//! Dynamic resolution: keeps the compositor inside the frame budget by
//! scaling the render target, the way games hold 60 fps on weaker GPUs.
//!
//! heartbeat.wgsl is one large per-pixel fragment shader, so frame time
//! grows with pixel count, i.e. with scale². Measured on a Galaxy S25 Ultra
//! (Chrome, 678x786 CSS px at DPR 2): 9 ms at 25 %, 14 ms at 50 %,
//! 16 ms at 75 %, 46 ms (queue backed up) at 100 %.

/// Frame budget at 60 Hz with headroom for the browser's own compositing.
pub const DEFAULT_BUDGET_MS: f64 = 13.0;
/// Below this share of the budget the scale is allowed to climb back.
const RAISE_BELOW: f64 = 0.6;
/// Frames averaged per decision (~0.5 s at 60 fps).
const WINDOW: u32 = 30;
/// Frames ignored after a change while the GPU queue settles.
const COOLDOWN: u32 = 20;
const STEP: f32 = 0.05;
const RAISE_FACTOR: f64 = 1.1;

#[derive(Debug, Clone)]
pub struct AutoScale {
    scale: f32,
    min: f32,
    max: f32,
    budget_ms: f64,
    sum_ms: f64,
    samples: u32,
    cooldown: u32,
}

impl AutoScale {
    pub fn new(start: f32, min: f32, max: f32, budget_ms: f64) -> Self {
        Self {
            scale: quantize(start.clamp(min, max)),
            min,
            max,
            budget_ms,
            sum_ms: 0.0,
            samples: 0,
            cooldown: COOLDOWN,
        }
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Feeds one GPU frame time. Returns the new scale when it changes.
    pub fn sample(&mut self, gpu_ms: f64) -> Option<f32> {
        if !(gpu_ms > 0.0 && gpu_ms.is_finite()) {
            return None;
        }
        if self.cooldown > 0 {
            self.cooldown -= 1;
            return None;
        }
        self.sum_ms += gpu_ms;
        self.samples += 1;
        if self.samples < WINDOW {
            return None;
        }
        let average = self.sum_ms / f64::from(self.samples);
        self.sum_ms = 0.0;
        self.samples = 0;

        let target = if average > self.budget_ms {
            // Time ~ pixels ~ scale², so this lands on the budget in one step;
            // the 0.6 floor stops a queue spike from collapsing the picture.
            let factor = (self.budget_ms / average).sqrt().clamp(0.6, 0.95);
            self.scale * factor as f32
        } else if average < self.budget_ms * RAISE_BELOW {
            self.scale * RAISE_FACTOR as f32
        } else {
            return None;
        };
        let next = quantize(target.clamp(self.min, self.max));
        if (next - self.scale).abs() < STEP / 2.0 {
            return None;
        }
        self.scale = next;
        self.cooldown = COOLDOWN;
        Some(next)
    }
}

fn quantize(scale: f32) -> f32 {
    (scale / STEP).round() * STEP
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GPU time model from the S25 Ultra measurements: fixed cost plus a
    /// per-pixel cost that grows with scale².
    fn phone_ms(scale: f32) -> f64 {
        7.0 + 13.0 * f64::from(scale * scale)
    }

    fn run(auto: &mut AutoScale, frames: usize, model: impl Fn(f32) -> f64) {
        for _ in 0..frames {
            let ms = model(auto.scale());
            auto.sample(ms);
        }
    }

    #[test]
    fn heavy_gpu_settles_inside_the_budget() {
        let mut auto = AutoScale::new(1.0, 0.25, 1.0, DEFAULT_BUDGET_MS);
        run(&mut auto, 1200, phone_ms);
        let settled = auto.scale();
        assert!(phone_ms(settled) <= DEFAULT_BUDGET_MS, "{settled} -> {} ms", phone_ms(settled));
        assert!(settled >= 0.5, "should not drop further than needed: {settled}");
    }

    #[test]
    fn steady_state_does_not_oscillate() {
        let mut auto = AutoScale::new(1.0, 0.25, 1.0, DEFAULT_BUDGET_MS);
        run(&mut auto, 1200, phone_ms);
        let settled = auto.scale();
        let mut changes = 0;
        for _ in 0..1200 {
            if auto.sample(phone_ms(auto.scale())).is_some() {
                changes += 1;
            }
        }
        assert_eq!(changes, 0, "scale kept moving around {settled}");
    }

    #[test]
    fn fast_gpu_climbs_back_to_full_resolution() {
        let mut auto = AutoScale::new(0.5, 0.25, 1.0, DEFAULT_BUDGET_MS);
        run(&mut auto, 2000, |scale| 2.0 + 4.0 * f64::from(scale * scale));
        assert_eq!(auto.scale(), 1.0);
    }

    #[test]
    fn a_queue_spike_drops_at_most_forty_percent_per_step() {
        let mut auto = AutoScale::new(1.0, 0.25, 1.0, DEFAULT_BUDGET_MS);
        let mut first = None;
        for _ in 0..200 {
            if let Some(next) = auto.sample(500.0) {
                first = Some(next);
                break;
            }
        }
        assert_eq!(first, Some(0.6));
    }

    #[test]
    fn ignores_missing_samples() {
        let mut auto = AutoScale::new(1.0, 0.25, 1.0, DEFAULT_BUDGET_MS);
        for _ in 0..500 {
            assert_eq!(auto.sample(0.0), None);
            assert_eq!(auto.sample(f64::NAN), None);
        }
        assert_eq!(auto.scale(), 1.0);
    }
}

//! Platform-free scene state: the JSON commands the editor already sends the
//! native core (`upsert_layer`, `set_layer_color`, …), applied to a small
//! layer table and packed into the byte layout heartbeat.wgsl reads.
//!
//! Field names and defaults follow native-renderer/src/main.rs
//! (`apply_upsert_layer`, `SceneLayer::new`, `SceneLayer::gpu`) so the same
//! command stream drives both cores. Only the subset this prototype draws is
//! handled; unknown commands are counted, not fatal.

use std::collections::HashMap;

use serde_json::Value;

use crate::compositor::{blend_mode_code, effect_descriptor_code};
use crate::layout::{CompositorLayout, StructWriter};

/// `SceneLayer::new` corners: TL, TR, BR, BL in output UV, y up.
const DEFAULT_CORNERS: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
/// `default_native_params()` in main.rs.
const DEFAULT_PARAMS: [f32; 8] = [1.0, 1.0, 0.5, 0.125, 0.0, 0.5, 0.25, 1.0];
const DEFAULT_SHAPE: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const DEFAULT_SHAPE2: [f32; 4] = [1.0, 0.7, 6.0, 0.4];
/// heartbeat.wgsl LayerData has four effect slots (effect0..effect3).
const MAX_LAYER_EFFECTS: usize = 4;
/// blend_mode_code("hierarchy-mask"): never a plain fill.
const HIERARCHY_MASK_BLEND: f32 = 26.0;

/// Same mapping as `source_kind()` in main.rs.
pub fn source_kind(source_type: &str) -> f32 {
    if source_type.starts_with("gpu:") {
        return 9.0;
    }
    match source_type {
        "color" => 1.0,
        "shader" => 2.0,
        "video" => 3.0,
        "image" => 4.0,
        // NATIVE_SHADER_SOURCE_KIND: the compositor's built-in ISF proxy.
        "isf-proxy" => 17.0,
        _ => 0.0,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub id: String,
    pub z_index: i32,
    pub visible: bool,
    pub opacity: f32,
    pub color: [f32; 4],
    pub tint: [f32; 4],
    pub source_kind: f32,
    pub corners: [[f32; 2]; 4],
    pub params: [f32; 8],
    pub blend_code: f32,
    pub effects: Vec<[f32; 4]>,
}

impl Layer {
    pub fn new(id: String, z_index: i32) -> Self {
        Self {
            id,
            z_index,
            visible: true,
            opacity: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            tint: [1.0; 4],
            source_kind: source_kind("color"),
            corners: DEFAULT_CORNERS,
            params: DEFAULT_PARAMS,
            blend_code: 0.0,
            effects: Vec::new(),
        }
    }

    /// `plain_fill` from `SceneLayer::gpu`: an untransformed solid rectangle
    /// the shader can take a fast path for.
    fn plain_fill(&self) -> bool {
        let [tl, tr, br, bl] = self.corners;
        self.source_kind < 9.0
            && self.effects.is_empty()
            && self.blend_code != HIERARCHY_MASK_BLEND
            && tl[1] == tr[1]
            && bl[1] == br[1]
            && tl[0] == bl[0]
            && tr[0] == br[0]
            && (br[0] - tl[0]).abs() > 0.000001
            && (br[1] - tl[1]).abs() > 0.000001
    }

    fn write_gpu(&self, out: &mut StructWriter) {
        let [tl, tr, br, bl] = self.corners;
        let alpha = self.color[3].clamp(0.0, 1.0);
        out.vec4("p0", [tl[0], tl[1], tr[0], tr[1]]);
        out.vec4("p1", [br[0], br[1], bl[0], bl[1]]);
        out.vec4("color", [self.color[0], self.color[1], self.color[2], self.opacity.clamp(0.0, 1.0) * alpha]);
        // `info` in WGSL, `meta` in the Rust mirror. No frame/preview slots yet.
        out.vec4("info", [if self.visible { 1.0 } else { 0.0 }, self.z_index as f32, self.source_kind, 0.0]);
        out.vec4("params0", [self.params[0], self.params[1], self.params[2], self.params[3]]);
        out.vec4("params1", [self.params[4], self.params[5], self.params[6], self.params[7]]);
        out.vec4("style", [self.blend_code, self.effects.len() as f32, 0.0, 0.0]);
        out.vec4("uv0", [0.0, 0.0, 1.0, 1.0]);
        out.vec4("uv1", [0.0, 1.0, 0.0, 0.0]);
        out.vec4("shape", DEFAULT_SHAPE);
        out.vec4("shape2", DEFAULT_SHAPE2);
        for (index, effect) in self.effects.iter().take(MAX_LAYER_EFFECTS).enumerate() {
            out.vec4(&format!("effect{index}"), *effect);
        }
        out.vec4("source_rect", [0.0, 0.0, 1.0, 1.0]);
        out.uvec4("fast_flags", [u32::from(self.plain_fill()), 0, 0, 0]);
        out.vec4("tint", [self.tint[0], self.tint[1], self.tint[2], alpha]);
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Audio {
    pub level: f32,
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    pub high: f32,
    pub beat: f32,
    pub beat_phase: f32,
    pub bpm: f32,
    pub centroid: f32,
    pub kick: f32,
    pub snare: f32,
    pub active: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ApplyReport {
    pub applied: usize,
    pub ignored: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Scene {
    layers: HashMap<String, Layer>,
    pub audio: Audio,
    /// (beat position at `anchor_time`, bpm, anchor time in seconds)
    beat_clock: (f64, f64, f64),
    pub output_gate: bool,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            layers: HashMap::new(),
            audio: Audio::default(),
            beat_clock: (0.0, 120.0, 0.0),
            output_gate: true,
        }
    }
}

impl Scene {
    pub fn layer(&self, id: &str) -> Option<&Layer> {
        self.layers.get(id)
    }

    /// Layers in draw order: z_index, then id so equal z is stable.
    pub fn ordered_layers(&self) -> Vec<&Layer> {
        let mut layers: Vec<&Layer> = self.layers.values().collect();
        layers.sort_by(|a, b| a.z_index.cmp(&b.z_index).then_with(|| a.id.cmp(&b.id)));
        layers
    }

    /// Accepts one command object, an array of them, or `{"commands": [...]}`,
    /// matching the batches the Electron broker writes to the native core.
    pub fn apply_json(&mut self, json: &str, now_seconds: f64) -> Result<ApplyReport, String> {
        let value: Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
        let commands = match value {
            Value::Array(items) => items,
            Value::Object(ref map) if map.contains_key("commands") => {
                map["commands"].as_array().cloned().unwrap_or_default()
            }
            other => vec![other],
        };
        let mut report = ApplyReport::default();
        for command in &commands {
            if self.apply(command, now_seconds) {
                report.applied += 1;
            } else {
                let kind = command.get("type").and_then(Value::as_str).unwrap_or("<no type>");
                report.ignored.push(kind.to_string());
            }
        }
        Ok(report)
    }

    fn layer_mut(&mut self, command: &Value) -> Option<&mut Layer> {
        let id = string_at(command, "layer_id")?;
        Some(self.layers.entry(id.clone()).or_insert_with(|| Layer::new(id, 0)))
    }

    fn apply(&mut self, command: &Value, now_seconds: f64) -> bool {
        let kind = command.get("type").and_then(Value::as_str).unwrap_or_default();
        match kind {
            "upsert_layer" => {
                let z_index = number_at(command, "z_index").unwrap_or(0.0).round() as i32;
                let Some(layer) = self.layer_mut(command) else { return false };
                layer.z_index = z_index;
                if let Some(opacity) = number_at(command, "opacity") {
                    layer.opacity = opacity.clamp(0.0, 1.0) as f32;
                }
                if let Some(blend) = string_at(command, "blend_mode") {
                    layer.blend_code = blend_mode_code(&blend);
                }
                if let Some(corners) = corners_at(command.get("corners")) {
                    layer.corners = corners;
                }
                if let Some(visible) = command.get("visible").and_then(Value::as_bool) {
                    layer.visible = visible;
                }
                true
            }
            "set_layer_visibility" => {
                let visible = command.get("visible").and_then(Value::as_bool).unwrap_or(true);
                self.layer_mut(command).map(|layer| layer.visible = visible).is_some()
            }
            "set_layer_color" | "set_layer_tint" => {
                let Some(rgba) = rgba_at(command.get("rgba")) else { return false };
                let tint = kind == "set_layer_tint";
                self.layer_mut(command)
                    .map(|layer| if tint { layer.tint = [rgba[0], rgba[1], rgba[2], 1.0] } else { layer.color = rgba })
                    .is_some()
            }
            "set_layer_native_params" => {
                let Some(params) = params8_at(command.get("params")) else { return false };
                self.layer_mut(command).map(|layer| layer.params = params).is_some()
            }
            // Web-core only: the native core binds sources through its media
            // pipeline (frame slots, graph layers). Here a layer just picks
            // which compositor branch draws it.
            "set_layer_source" => {
                let Some(source_type) = string_at(command, "source_type") else { return false };
                self.layer_mut(command).map(|layer| layer.source_kind = source_kind(&source_type)).is_some()
            }
            "set_layer_effects" => {
                let effects = command
                    .get("effects")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .filter_map(effect_descriptor_code)
                            .take(MAX_LAYER_EFFECTS)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                self.layer_mut(command).map(|layer| layer.effects = effects).is_some()
            }
            "remove_layer" => string_at(command, "layer_id")
                .map(|id| self.layers.remove(&id).is_some())
                .unwrap_or(false),
            "clear_layers" => {
                self.layers.clear();
                true
            }
            "set_audio_state" => {
                let f = |key: &str| number_at(command, key).unwrap_or(0.0) as f32;
                self.audio = Audio {
                    level: f("level"),
                    bass: f("bass"),
                    mid: f("mid"),
                    treble: f("treble"),
                    high: f("high"),
                    beat: f("beat"),
                    beat_phase: f("beat_phase"),
                    bpm: f("bpm"),
                    centroid: f("centroid"),
                    kick: f("kick"),
                    snare: f("snare"),
                    active: command.get("active").and_then(Value::as_bool).unwrap_or(true),
                };
                true
            }
            "set_beat_clock" => {
                let bpm = number_at(command, "bpm").or_else(|| number_at(command, "tempo")).unwrap_or(120.0);
                let beat = number_at(command, "beat").unwrap_or(0.0);
                self.beat_clock = (beat, bpm.clamp(20.0, 999.0), now_seconds);
                true
            }
            "set_output_gate" => {
                self.output_gate = command.get("enabled").and_then(Value::as_bool).unwrap_or(true);
                true
            }
            _ => false,
        }
    }

    pub fn beat_at(&self, now_seconds: f64) -> (f32, f32) {
        let (beat, bpm, anchor) = self.beat_clock;
        let position = beat + (now_seconds - anchor).max(0.0) * bpm / 60.0;
        // BEAT_CLOCK_WRAP in main.rs keeps sub-beat precision in an f32.
        ((position % 65520.0) as f32, bpm as f32)
    }

    /// Packs the visible draw list into `LayerData` structs. Returns the bytes
    /// and the layer count (at least one zeroed struct so the storage binding
    /// is never empty).
    pub fn pack_layers(&self, layout: &CompositorLayout) -> (Vec<u8>, usize) {
        let stride = layout.layer.size as usize;
        let layers = self.ordered_layers();
        let mut bytes = vec![0u8; stride * layers.len().max(1)];
        for (index, layer) in layers.iter().enumerate() {
            let slot = &mut bytes[index * stride..(index + 1) * stride];
            layer.write_gpu(&mut StructWriter::new(&layout.layer, slot));
        }
        (bytes, layers.len())
    }

    pub fn pack_uniforms(
        &self,
        layout: &CompositorLayout,
        resolution: [f32; 2],
        time_seconds: f64,
        frame: u64,
        layer_count: usize,
    ) -> Vec<u8> {
        let mut bytes = vec![0u8; layout.uniforms.size as usize];
        let mut u = StructWriter::new(&layout.uniforms, &mut bytes);
        u.vec2("resolution", resolution);
        u.f32("time", time_seconds as f32);
        u.f32("layer_count", layer_count as f32);
        u.f32("frame_count", frame as f32);
        u.f32("output_gate", if self.output_gate { 1.0 } else { 0.0 });
        let a = &self.audio;
        u.vec4("audio0", [a.level, a.bass, a.mid, a.treble]);
        u.vec4("audio1", [a.high, a.beat, a.beat_phase, a.bpm]);
        u.vec4("audio2", [a.centroid, a.kick, a.snare, if a.active { 1.0 } else { 0.0 }]);
        // OutputStage::default() in main.rs: identity crop, grade and warps.
        u.vec4("out0", [0.0, 0.0, 1.0, 1.0]);
        u.vec4("out1", [0.0, 1.0, 1.0, 1.0]);
        u.vec4("dome2", [1.0, 2.2, 0.0, 0.0]);
        u.vec4("edge_gamma", [2.2; 4]);
        u.vec4("swarp_c0", [0.0, 0.0, 1.0, 0.0]);
        u.vec4("swarp_c1", [1.0, 1.0, 0.0, 1.0]);
        u.vec4("mwarp_c0", [0.0, 0.0, 1.0, 0.0]);
        u.vec4("mwarp_c1", [1.0, 1.0, 0.0, 1.0]);
        let (beat, bpm) = self.beat_at(time_seconds);
        u.vec4("clock", [beat, bpm, 0.0, 0.0]);
        bytes
    }
}

fn string_at(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(str::to_string)
}

fn number_at(value: &Value, key: &str) -> Option<f64> {
    value.get(key)?.as_f64().filter(|number| number.is_finite())
}

fn rgba_at(value: Option<&Value>) -> Option<[f32; 4]> {
    let items = value?.as_array()?;
    if items.len() < 4 {
        return None;
    }
    let mut rgba = [0.0; 4];
    for (slot, item) in rgba.iter_mut().zip(items) {
        *slot = item.as_f64()?.clamp(0.0, 1.0) as f32;
    }
    Some(rgba)
}

fn params8_at(value: Option<&Value>) -> Option<[f32; 8]> {
    let items = value?.as_array()?;
    let mut params = DEFAULT_PARAMS;
    for (slot, item) in params.iter_mut().zip(items) {
        *slot = item.as_f64()? as f32;
    }
    Some(params)
}

/// `corners_at` in main.rs: {topLeft, topRight, bottomRight, bottomLeft}.
fn corners_at(value: Option<&Value>) -> Option<[[f32; 2]; 4]> {
    let value = value?;
    let point = |key: &str| -> Option<[f32; 2]> {
        let point = value.get(key)?;
        Some([
            point.get("x")?.as_f64()?.clamp(-8.0, 8.0) as f32,
            point.get("y")?.as_f64()?.clamp(-8.0, 8.0) as f32,
        ])
    };
    Some([point("topLeft")?, point("topRight")?, point("bottomRight")?, point("bottomLeft")?])
}

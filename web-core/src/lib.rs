//! Ghost Arcade web core (prototype).
//!
//! Runs the native render core's compositor shader, heartbeat.wgsl, in the
//! browser through WebAssembly + WebGPU, driven by the same JSON commands the
//! editor sends the native process. Nothing under native-renderer/ is
//! modified: the shader and the pure helpers in compositor.rs are included
//! by path.
//!
//! JS API (see web-core/demo/):
//!   const core = await WebCore.create(canvas);
//!   core.apply('[{"type":"upsert_layer", ...}]');
//!   core.render(performance.now() / 1000);

#[path = "../../native-renderer/src/compositor.rs"]
#[allow(dead_code)]
mod compositor;
pub mod layout;
pub mod scene;

/// The native compositor shader, verbatim.
pub const HEARTBEAT_WGSL: &str = include_str!("../../native-renderer/src/heartbeat.wgsl");

/// Browsers (Tint) reject `fwidth` in non-uniform control flow, which naga
/// on the desktop accepts. heartbeat.wgsl uses it inside per-layer branches,
/// so the web build turns that one diagnostic off instead of editing the
/// shared shader. Supported in WGSL since Chrome 116 / Safari 26.
pub fn browser_wgsl() -> String {
    format!("diagnostic(off, derivative_uniformity);\n{HEARTBEAT_WGSL}")
}

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(test)]
mod tests;

# Ghost Web Core (prototype)

The native render core's compositor running in the browser: Rust compiled to
WebAssembly, drawing through WebGPU with the **unmodified**
`native-renderer/src/heartbeat.wgsl`.

```
editor JSON commands ──► WebCore.apply()  ──► Scene (scene.rs)
                                                │ pack into the shader's own layout
                                                ▼ (layout.rs reads it from heartbeat.wgsl via naga)
performance.now() ─────► WebCore.render() ──► WebGPU: heartbeat.wgsl fs_main ──► <canvas>
```

## Build and run

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked   # must match Cargo.lock
./web-core/build.sh                                          # → web-core/demo/pkg/
npx serve web-core/demo                                      # any static server
```

Open the page in Chrome or Edge (macOS, Windows, Android), or Safari on
macOS/iOS 26. Tests run natively: `cargo test --manifest-path web-core/Cargo.toml`.

## How it stays merge-friendly

- `native-renderer/` is not edited. The shader is `include_str!`'d and the
  pure helpers (`blend_mode_code`, `effect_descriptor_code`) come from
  `compositor.rs` by `#[path]`.
- Struct offsets come from naga at startup, not from a copied Rust mirror of
  `Uniforms` / `LayerData`, so a field upstream adds is left zeroed instead of
  shifting every offset after it.
- One browser-only difference: Tint rejects `fwidth` in non-uniform control
  flow, which naga accepts. The web core prepends
  `diagnostic(off, derivative_uniformity);` rather than changing the shader.

## What works

| Feature | Status |
|---|---|
| Colour layers, z-order, opacity, visibility | yes |
| All 26 blend modes (`blend_mode_code`) | yes |
| Corner pin (`corners`) | yes |
| Layer effects: invert, grayscale, brightness, contrast, gamma, saturation, hue, posterize, noise (up to 4) | yes |
| GPU instrument proxies (`gpu:*`) and the built-in ISF proxy | yes (procedural previews from heartbeat.wgsl) |
| Audio uniforms, beat clock, output gate (blackout) | yes |
| Commands: `upsert_layer`, `set_layer_visibility`, `set_layer_color`, `set_layer_tint`, `set_layer_native_params`, `remove_layer`, `clear_layers`, `set_audio_state`, `set_beat_clock` + web-only `set_layer_source`, `set_layer_effects`, `set_output_gate` | yes |

## Verified

In headless Chromium with WebGPU (SwiftShader): the page renders at 60 fps,
a full-screen red layer reads back as (143, 0, 0) because heartbeat.wgsl
draws plain colour fills at 56 % alpha (`color.a * 0.56`), exactly as the
native core does, and `set_output_gate` false reads back pure black.
Not yet tried on real GPUs (Mac Chrome, Android Chrome, iOS 26 Safari).

## Not yet

- Video and image sources (frame slots): needs `<video>`/WebCodecs →
  `copyExternalImageToTexture` into the `source_frames` array.
- Real ISF shaders (`isf_passes.rs`), compute graphs, particle instruments.
- Masks, meshes/Bezier, Edge Effects, paint masks, output warps and slices.
- Wiring into the editor: today the demo page sends commands by hand.

## Size

`ghost_web_core.wasm` is about 2.6 MB uncompressed, most of it naga (used
only to read struct offsets). Computing the layout at build time would drop
naga from the runtime.

use crate::layout::{CompositorLayout, compositor_layout};
use crate::scene::{Scene, source_kind};
use crate::{HEARTBEAT_WGSL, browser_wgsl};

fn layout() -> CompositorLayout {
    compositor_layout(HEARTBEAT_WGSL).expect("heartbeat.wgsl parses")
}

fn read_vec4(bytes: &[u8], offset: u32) -> [f32; 4] {
    let start = offset as usize;
    *bytemuck::from_bytes(&bytes[start..start + 16])
}

#[test]
fn layout_finds_every_member_the_scene_writes() {
    let layout = layout();
    for member in [
        "resolution", "time", "layer_count", "frame_count", "output_gate", "audio0", "audio1", "audio2",
        "out0", "out1", "dome2", "edge_gamma", "swarp_c0", "swarp_c1", "mwarp_c0", "mwarp_c1", "clock",
    ] {
        assert!(layout.uniforms.has(member), "Uniforms.{member} missing");
    }
    for member in [
        "p0", "p1", "color", "info", "params0", "params1", "style", "uv0", "uv1", "shape", "shape2",
        "effect0", "effect3", "source_rect", "fast_flags", "tint",
    ] {
        assert!(layout.layer.has(member), "LayerData.{member} missing");
    }
}

#[test]
fn layout_fits_webgpu_default_limits() {
    let layout = layout();
    // WebGPU guarantees maxUniformBufferBindingSize >= 65536 on every adapter.
    assert!(layout.uniforms.size <= 65_536, "uniforms are {} bytes", layout.uniforms.size);
    assert_eq!(layout.layer.size % 16, 0);
}

#[test]
fn browser_shader_keeps_the_native_source_intact() {
    let wgsl = browser_wgsl();
    assert!(wgsl.starts_with("diagnostic(off, derivative_uniformity);\n"));
    assert!(wgsl.ends_with(HEARTBEAT_WGSL));
    // The directive must still parse with the same naga the native core uses.
    naga::front::wgsl::parse_str(&wgsl).expect("directive parses");
}

#[test]
fn upsert_and_color_pack_into_the_shader_offsets() {
    let layout = layout();
    let mut scene = Scene::default();
    let report = scene
        .apply_json(
            r#"[
              {"type":"upsert_layer","layer_id":"a","z_index":2,"opacity":0.5,"blend_mode":"screen",
               "corners":{"topLeft":{"x":0.1,"y":0.9},"topRight":{"x":0.9,"y":0.9},
                          "bottomRight":{"x":0.9,"y":0.1},"bottomLeft":{"x":0.1,"y":0.1}}},
              {"type":"set_layer_color","layer_id":"a","rgba":[1,0,0,0.8]},
              {"type":"upsert_layer","layer_id":"b","z_index":1},
              {"type":"set_layer_source","layer_id":"b","source_type":"gpu:planet"},
              {"type":"set_layer_effects","layer_id":"b","effects":["invert:1","bogus","hue:0.5"]},
              {"type":"no_such_command"}
            ]"#,
            0.0,
        )
        .unwrap();
    assert_eq!(report.applied, 5);
    assert_eq!(report.ignored, vec!["no_such_command".to_string()]);

    let (bytes, count) = scene.pack_layers(&layout);
    assert_eq!(count, 2);
    let stride = layout.layer.size;
    let off = |member: &str| layout.layer.offset(member).unwrap();

    // Draw order is z ascending: "b" (z 1) first, "a" (z 2) second.
    let b = &bytes[..stride as usize];
    assert_eq!(read_vec4(b, off("info"))[2], source_kind("gpu:planet"));
    assert_eq!(read_vec4(b, off("style"))[1], 2.0, "two valid effects");
    assert_eq!(read_vec4(b, off("effect0"))[0], 1.0, "invert op code");
    assert_eq!(read_vec4(b, off("effect1"))[0], 7.0, "hue op code");

    let a = &bytes[stride as usize..];
    assert_eq!(read_vec4(a, off("p0")), [0.1, 0.9, 0.9, 0.9]);
    assert_eq!(read_vec4(a, off("p1")), [0.9, 0.1, 0.1, 0.1]);
    let color = read_vec4(a, off("color"));
    assert_eq!(&color[..3], &[1.0, 0.0, 0.0]);
    assert!((color[3] - 0.4).abs() < 1e-6, "opacity 0.5 x alpha 0.8");
    assert_eq!(read_vec4(a, off("style"))[0], 3.0, "screen blend code");
    let fast_flags: [u32; 4] = *bytemuck::from_bytes(&a[off("fast_flags") as usize..off("fast_flags") as usize + 16]);
    assert_eq!(fast_flags[0], 1, "axis-aligned colour rect is a plain fill");
}

#[test]
fn uniforms_carry_output_identity_audio_and_beat_clock() {
    let layout = layout();
    let mut scene = Scene::default();
    scene
        .apply_json(
            r#"[{"type":"set_audio_state","level":0.5,"bass":0.25,"beat":1},
                {"type":"set_beat_clock","beat":4,"bpm":120}]"#,
            10.0,
        )
        .unwrap();
    let bytes = scene.pack_uniforms(&layout, [1920.0, 1080.0], 11.0, 7, 3);
    let off = |member: &str| layout.uniforms.offset(member).unwrap();
    assert_eq!(read_vec4(&bytes, off("out0")), [0.0, 0.0, 1.0, 1.0]);
    assert_eq!(read_vec4(&bytes, off("out1")), [0.0, 1.0, 1.0, 1.0]);
    assert_eq!(read_vec4(&bytes, off("audio0"))[..2], [0.5, 0.25]);
    assert_eq!(read_vec4(&bytes, off("audio2"))[3], 1.0, "audio active");
    // One second after the anchor at 120 bpm: beat 4 + 2.
    assert_eq!(read_vec4(&bytes, off("clock"))[..2], [6.0, 120.0]);
    let gate: f32 = *bytemuck::from_bytes(&bytes[off("output_gate") as usize..off("output_gate") as usize + 4]);
    assert_eq!(gate, 1.0);
}

#[test]
fn remove_and_clear_layers() {
    let mut scene = Scene::default();
    scene.apply_json(r#"[{"type":"upsert_layer","layer_id":"a"},{"type":"upsert_layer","layer_id":"b"}]"#, 0.0).unwrap();
    scene.apply_json(r#"{"type":"remove_layer","layer_id":"a"}"#, 0.0).unwrap();
    assert!(scene.layer("a").is_none() && scene.layer("b").is_some());
    scene.apply_json(r#"{"commands":[{"type":"clear_layers"}]}"#, 0.0).unwrap();
    assert!(scene.ordered_layers().is_empty());
    assert!(scene.apply_json("not json", 0.0).is_err());
}

#[test]
fn empty_scene_still_packs_one_struct() {
    let layout = layout();
    let (bytes, count) = Scene::default().pack_layers(&layout);
    assert_eq!(count, 0);
    assert_eq!(bytes.len(), layout.layer.size as usize);
}

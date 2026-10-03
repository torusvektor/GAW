//! Byte layout of the compositor's GPU structs, read from heartbeat.wgsl.
//!
//! The native core mirrors `Uniforms` and `LayerData` as `#[repr(C)]` Rust
//! structs inside its 33k-line main.rs. Instead of copying those structs
//! (and drifting the next time upstream adds a field), this crate asks naga
//! for the offsets of the shader's own declarations. A field upstream adds
//! later is simply left zeroed until the web core learns to fill it.

use std::collections::HashMap;

use naga::{Module, TypeInner};

#[derive(Debug, Clone)]
pub struct StructLayout {
    /// Size of one struct in bytes (for `layers`, the array stride).
    pub size: u32,
    offsets: HashMap<String, u32>,
}

impl StructLayout {
    pub fn offset(&self, member: &str) -> Option<u32> {
        self.offsets.get(member).copied()
    }

    pub fn has(&self, member: &str) -> bool {
        self.offsets.contains_key(member)
    }
}

#[derive(Debug, Clone)]
pub struct CompositorLayout {
    /// `var<uniform> u: Uniforms`
    pub uniforms: StructLayout,
    /// One element of `var<storage, read> layers: array<LayerData>`.
    pub layer: StructLayout,
}

pub fn compositor_layout(wgsl: &str) -> Result<CompositorLayout, String> {
    let module = naga::front::wgsl::parse_str(wgsl).map_err(|error| error.emit_to_string(wgsl))?;
    let mut layouter = naga::proc::Layouter::default();
    layouter
        .update(module.to_ctx())
        .map_err(|error| format!("layout: {error:?}"))?;

    let uniform_ty = global_type(&module, "u")?;
    let uniforms = struct_layout(&module, uniform_ty, layouter[uniform_ty].size)?;

    let layers_ty = global_type(&module, "layers")?;
    let TypeInner::Array { base, stride, .. } = module.types[layers_ty].inner else {
        return Err("`layers` is not an array".into());
    };
    let layer = struct_layout(&module, base, stride)?;

    Ok(CompositorLayout { uniforms, layer })
}

fn global_type(module: &Module, name: &str) -> Result<naga::Handle<naga::Type>, String> {
    module
        .global_variables
        .iter()
        .find(|(_, global)| global.name.as_deref() == Some(name))
        .map(|(_, global)| global.ty)
        .ok_or_else(|| format!("global `{name}` not found in heartbeat.wgsl"))
}

fn struct_layout(module: &Module, ty: naga::Handle<naga::Type>, size: u32) -> Result<StructLayout, String> {
    let TypeInner::Struct { ref members, .. } = module.types[ty].inner else {
        return Err(format!("{:?} is not a struct", module.types[ty].name));
    };
    let offsets = members
        .iter()
        .filter_map(|member| member.name.clone().map(|name| (name, member.offset)))
        .collect();
    Ok(StructLayout { size, offsets })
}

/// Zero-initialised bytes for one struct, written field by field.
pub struct StructWriter<'a> {
    layout: &'a StructLayout,
    bytes: &'a mut [u8],
}

impl<'a> StructWriter<'a> {
    pub fn new(layout: &'a StructLayout, bytes: &'a mut [u8]) -> Self {
        debug_assert!(bytes.len() >= layout.size as usize);
        Self { layout, bytes }
    }

    /// Writes `values` at `member`'s offset plus `index` 16-byte slots
    /// (arrays of vec4 have a 16-byte stride). Unknown members are skipped,
    /// so an older or newer heartbeat.wgsl still renders what it can.
    fn write(&mut self, member: &str, index: usize, values: &[u8]) {
        let Some(offset) = self.layout.offset(member) else { return };
        let start = offset as usize + index * 16;
        let end = start + values.len();
        if end <= self.bytes.len() {
            self.bytes[start..end].copy_from_slice(values);
        }
    }

    pub fn f32(&mut self, member: &str, value: f32) {
        self.write(member, 0, bytemuck::bytes_of(&value));
    }

    pub fn vec2(&mut self, member: &str, value: [f32; 2]) {
        self.write(member, 0, bytemuck::bytes_of(&value));
    }

    pub fn vec4(&mut self, member: &str, value: [f32; 4]) {
        self.write(member, 0, bytemuck::bytes_of(&value));
    }

    pub fn vec4_at(&mut self, member: &str, index: usize, value: [f32; 4]) {
        self.write(member, index, bytemuck::bytes_of(&value));
    }

    pub fn uvec4(&mut self, member: &str, value: [u32; 4]) {
        self.write(member, 0, bytemuck::bytes_of(&value));
    }
}

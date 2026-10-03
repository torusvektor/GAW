//! WebGPU renderer and the wasm-bindgen surface. Browser-only.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wgpu::util::DeviceExt;

use crate::layout::{CompositorLayout, compositor_layout};
use crate::scene::Scene;

/// heartbeat.wgsl reads `source_previews` only through preview slots, which
/// the prototype never assigns; a small buffer satisfies the binding.
const PREVIEW_BUFFER_BYTES: u64 = 4096;

#[wasm_bindgen]
pub struct WebCore {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    layer_buffer: wgpu::Buffer,
    preview_buffer: wgpu::Buffer,
    source_frames: wgpu::TextureView,
    paint_masks: wgpu::TextureView,
    sampler: wgpu::Sampler,
    layout: CompositorLayout,
    scene: Scene,
    frame: u64,
    last_time: f64,
    adapter_info: String,
    /// Milliseconds from the last submit until the GPU reported it done
    /// (f64 bits; written from the queue's work-done callback).
    gpu_ms: Arc<AtomicU64>,
}

fn js_error(message: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&message.to_string())
}

fn texture_array_view(device: &wgpu::Device, label: &str, format: wgpu::TextureFormat) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        })
}

#[wasm_bindgen]
impl WebCore {
    /// Creates the WebGPU device and compositor pipeline for `canvas`.
    /// Rejects when the browser has no WebGPU adapter, so the caller can
    /// fall back to the WebGL editor.
    pub async fn create(canvas: HtmlCanvasElement) -> Result<WebCore, JsValue> {
        console_error_panic_hook::set_once();
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(js_error)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(|error| js_error(format!("WebGPU adapter unavailable: {error}")))?;
        let info = adapter.get_info();
        let adapter_info = format!("{} ({:?})", info.name, info.backend);

        let layout = compositor_layout(crate::HEARTBEAT_WGSL).map_err(js_error)?;
        let limits = adapter.limits();
        if layout.uniforms.size > limits.max_uniform_buffer_binding_size as u32 {
            return Err(js_error(format!(
                "compositor uniforms need {} bytes, adapter allows {}",
                layout.uniforms.size, limits.max_uniform_buffer_binding_size
            )));
        }
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Ghost Web Core"),
                required_limits: limits,
                ..Default::default()
            })
            .await
            .map_err(js_error)?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|format| !format.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            color_space: Default::default(),
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("heartbeat.wgsl"),
            source: wgpu::ShaderSource::Wgsl(crate::browser_wgsl().into()),
        });
        let visibility = wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT;
        let storage = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let texture_array = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        };
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("heartbeat bindings"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                storage(1),
                storage(2),
                texture_array(3),
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                texture_array(5),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("heartbeat layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            ..Default::default()
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("heartbeat fs_main"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heartbeat uniforms"),
            size: layout.uniforms.size as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layer_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("heartbeat layers"),
            contents: &vec![0u8; layout.layer.size as usize],
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let preview_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heartbeat source previews"),
            size: PREVIEW_BUFFER_BYTES,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let source_frames = texture_array_view(&device, "source frames", wgpu::TextureFormat::Rgba8Unorm);
        let paint_masks = texture_array_view(&device, "paint masks", wgpu::TextureFormat::R8Unorm);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("source frame sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bind_group = Self::make_bind_group(
            &device,
            &bind_group_layout,
            &uniform_buffer,
            &layer_buffer,
            &preview_buffer,
            &source_frames,
            &sampler,
            &paint_masks,
        );

        Ok(WebCore {
            device,
            queue,
            surface,
            config,
            pipeline,
            bind_group_layout,
            bind_group,
            uniform_buffer,
            layer_buffer,
            preview_buffer,
            source_frames,
            paint_masks,
            sampler,
            layout,
            scene: Scene::default(),
            frame: 0,
            last_time: 0.0,
            adapter_info,
            gpu_ms: Arc::new(AtomicU64::new(0)),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn make_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        uniforms: &wgpu::Buffer,
        layers: &wgpu::Buffer,
        previews: &wgpu::Buffer,
        source_frames: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        paint_masks: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("heartbeat bind group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: layers.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: previews.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(source_frames) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(sampler) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(paint_masks) },
            ],
        })
    }

    /// Applies a JSON command, an array of commands, or `{"commands": [...]}`.
    /// Returns the number applied; unknown command types are skipped.
    pub fn apply(&mut self, json: &str) -> Result<u32, JsValue> {
        let report = self.scene.apply_json(json, self.last_time).map_err(js_error)?;
        if !report.ignored.is_empty() {
            web_sys::console::warn_1(&format!("[web-core] ignored commands: {:?}", report.ignored).into());
        }
        Ok(report.applied as u32)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let (width, height) = (width.max(1), height.max(1));
        if width == self.config.width && height == self.config.height {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Draws one frame. `time_seconds` comes from JS (performance.now()),
    /// which also sidesteps std::time::Instant, unavailable in wasm32.
    pub fn render(&mut self, time_seconds: f64) -> Result<(), JsValue> {
        self.last_time = time_seconds;
        let (layer_bytes, layer_count) = self.scene.pack_layers(&self.layout);
        if layer_bytes.len() as u64 > self.layer_buffer.size() {
            // Grow to the next power of two so adding layers one by one does
            // not rebuild the buffer every frame.
            let size = (layer_bytes.len() as u64).next_power_of_two();
            self.layer_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("heartbeat layers"),
                size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.bind_group = Self::make_bind_group(
                &self.device,
                &self.bind_group_layout,
                &self.uniform_buffer,
                &self.layer_buffer,
                &self.preview_buffer,
                &self.source_frames,
                &self.sampler,
                &self.paint_masks,
            );
        }
        self.queue.write_buffer(&self.layer_buffer, 0, &layer_bytes);
        let uniforms = self.scene.pack_uniforms(
            &self.layout,
            [self.config.width as f32, self.config.height as f32],
            time_seconds,
            self.frame,
            layer_count,
        );
        self.queue.write_buffer(&self.uniform_buffer, 0, &uniforms);

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            other => return Err(js_error(format!("surface: {other:?}"))),
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("heartbeat"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        // Submit-to-done latency: GPU time plus queueing. Timestamp queries
        // would be exact but are an optional feature many phones lack.
        let submitted = now_ms();
        let gpu_ms = Arc::clone(&self.gpu_ms);
        self.queue.on_submitted_work_done(move || {
            gpu_ms.store((now_ms() - submitted).to_bits(), Ordering::Relaxed);
        });
        self.queue.present(frame);
        self.frame = self.frame.wrapping_add(1);
        Ok(())
    }

    /// Last measured submit-to-done time in milliseconds.
    #[wasm_bindgen(getter, js_name = gpuMs)]
    pub fn gpu_ms(&self) -> f64 {
        f64::from_bits(self.gpu_ms.load(Ordering::Relaxed))
    }

    #[wasm_bindgen(getter)]
    pub fn adapter(&self) -> String {
        self.adapter_info.clone()
    }

    #[wasm_bindgen(getter, js_name = layerCount)]
    pub fn layer_count(&self) -> u32 {
        self.scene.ordered_layers().len() as u32
    }

    /// Byte sizes the shader layout resolved to, for diagnostics.
    #[wasm_bindgen(getter, js_name = layoutInfo)]
    pub fn layout_info(&self) -> String {
        format!("uniforms {} B, layer stride {} B", self.layout.uniforms.size, self.layout.layer.size)
    }
}

fn now_ms() -> f64 {
    web_sys::window().and_then(|window| window.performance()).map(|performance| performance.now()).unwrap_or(0.0)
}

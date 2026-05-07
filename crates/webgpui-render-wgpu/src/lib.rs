//! wgpu-based renderer for webgpui.
//!
//! # Architecture
//! ```text
//! App → DrawList → Batcher → DrawBatch[]
//!                                ↓
//!                       WgpuRenderer::render()
//!                         ├── upload vertices/indices
//!                         ├── write globals uniform
//!                         └── RenderGraph passes (clear → ui)
//! ```

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use webgpui_batching::{Batcher, DrawBatch, Vertex, VERTEX_SIZE};
use webgpui_render::{DrawList, RenderError, RenderResult, Renderer};
use webgpui_render_graph::{ClearColor, PassKind, RenderGraph};

// ---------------------------------------------------------------------------
// WGSL shader (embedded)
// ---------------------------------------------------------------------------

const SHADER_SRC: &str = r#"
struct Globals {
    viewport_size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color:    vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Convert from pixel-space (origin top-left, y-down) to NDC.
    let ndc_x =  (in.position.x / globals.viewport_size.x) * 2.0 - 1.0;
    let ndc_y = -(in.position.y / globals.viewport_size.y) * 2.0 + 1.0;
    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

// ---------------------------------------------------------------------------
// Globals uniform
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Globals {
    viewport_size: [f32; 2],
    _pad: [f32; 2],
}

// ---------------------------------------------------------------------------
// ResolvedFrame – result of querying the RenderGraph for a frame
// ---------------------------------------------------------------------------

struct ResolvedFrame {
    clear_enabled: bool,
    clear_color: ClearColor,
    ui_enabled: bool,
}

// ---------------------------------------------------------------------------
// WgpuContext – device + surface
// ---------------------------------------------------------------------------

/// Owns the wgpu device, queue, and surface.
///
/// Create once, then hand off to [`WgpuRenderer`].
pub struct WgpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_format: wgpu::TextureFormat,
    pub config: wgpu::SurfaceConfiguration,
    pub adapter_info: wgpu::AdapterInfo,
}

impl WgpuContext {
    /// Initialises wgpu on the given raw window handle.
    ///
    /// `window` must outlive the returned `WgpuContext`.
    pub fn new(
        window: Arc<
            impl raw_window_handle::HasWindowHandle
                + raw_window_handle::HasDisplayHandle
                + Send
                + Sync
                + 'static,
        >,
        width: u32,
        height: u32,
        vsync: bool,
    ) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window).map_err(|e| e.to_string())?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| "no suitable GPU adapter found".to_string())?;

        let adapter_info = adapter.get_info();
        log::info!("[wgpu] adapter: {:?}", adapter_info);

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("webgpui-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        ))
        .map_err(|e| e.to_string())?;

        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .or_else(|| caps.formats.first().copied())
            .unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);

        let present_mode = if vsync {
            wgpu::PresentMode::Fifo
        } else {
            wgpu::PresentMode::Immediate
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: width.max(1),
            height: height.max(1),
            present_mode,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Ok(Self {
            device,
            queue,
            surface,
            surface_format,
            config,
            adapter_info,
        })
    }
}

// ---------------------------------------------------------------------------
// WgpuRenderer
// ---------------------------------------------------------------------------

/// Renders [`DrawList`]s to a wgpu surface.
pub struct WgpuRenderer {
    ctx: WgpuContext,
    pipeline: wgpu::RenderPipeline,
    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    /// Reusable vertex buffer (resized as needed).
    vertex_buffer: wgpu::Buffer,
    vertex_buffer_capacity: u64,
    /// Reusable index buffer (resized as needed).
    index_buffer: wgpu::Buffer,
    index_buffer_capacity: u64,
    batcher: Batcher,
    render_graph: RenderGraph,
    /// Staging buffers reused across frames to avoid per-frame allocation.
    staging_vertices: Vec<Vertex>,
    staging_indices: Vec<u32>,
    staging_batch_ranges: Vec<(u32, u32, u32)>,
}

impl WgpuRenderer {
    pub fn new(ctx: WgpuContext) -> Self {
        let device = &ctx.device;

        // Globals bind-group layout.
        let globals_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Globals buffer.
        let globals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("globals-buf"),
            contents: bytemuck::bytes_of(&Globals {
                viewport_size: [ctx.config.width as f32, ctx.config.height as f32],
                _pad: [0.0; 2],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals-bg"),
            layout: &globals_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buffer.as_entire_binding(),
            }],
        });

        // Shader.
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rect-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });

        // Pipeline layout.
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rect-pipeline-layout"),
            bind_group_layouts: &[&globals_bgl],
            push_constant_ranges: &[],
        });

        // Vertex buffer layout.
        let vbl = wgpu::VertexBufferLayout {
            array_stride: VERTEX_SIZE,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as u64,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };

        // Alpha pipeline.
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rect-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vbl],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: ctx.surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Pre-allocate buffers (1 MB each).
        const INITIAL_VBUF: u64 = 1024 * 1024;
        const INITIAL_IBUF: u64 = 1024 * 1024;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex-buf"),
            size: INITIAL_VBUF,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("index-buf"),
            size: INITIAL_IBUF,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            ctx,
            pipeline,
            globals_buffer,
            globals_bind_group,
            vertex_buffer,
            vertex_buffer_capacity: INITIAL_VBUF,
            index_buffer,
            index_buffer_capacity: INITIAL_IBUF,
            batcher: Batcher::new(),
            render_graph: RenderGraph::new(),
            staging_vertices: Vec::new(),
            staging_indices: Vec::new(),
            staging_batch_ranges: Vec::new(),
        }
    }

    /// Returns a mutable reference to the [`RenderGraph`], e.g. to adjust the
    /// clear colour.
    pub fn render_graph_mut(&mut self) -> &mut RenderGraph {
        &mut self.render_graph
    }

    // ------------------------------------------------------------------
    // Buffer management helpers
    // ------------------------------------------------------------------

    fn ensure_vertex_buffer(&mut self, needed: u64) {
        if needed > self.vertex_buffer_capacity {
            let new_size = needed.next_power_of_two().max(needed);
            self.vertex_buffer = self.ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("vertex-buf"),
                size: new_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.vertex_buffer_capacity = new_size;
            log::debug!("[wgpu] vertex buffer resized to {} bytes", new_size);
        }
    }

    fn ensure_index_buffer(&mut self, needed: u64) {
        if needed > self.index_buffer_capacity {
            let new_size = needed.next_power_of_two().max(needed);
            self.index_buffer = self.ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("index-buf"),
                size: new_size,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.index_buffer_capacity = new_size;
            log::debug!("[wgpu] index buffer resized to {} bytes", new_size);
        }
    }

    // ------------------------------------------------------------------
    // Per-frame rendering
    // ------------------------------------------------------------------

    /// Queries the [`RenderGraph`] and returns the resolved per-frame state.
    fn resolve_graph(&mut self) -> ResolvedFrame {
        let order = self.render_graph.execution_order();
        ResolvedFrame {
            clear_enabled: order.iter().any(|p| p.kind == PassKind::Clear),
            clear_color: order
                .iter()
                .find(|p| p.kind == PassKind::Clear)
                .map(|p| p.clear_color)
                .unwrap_or(ClearColor::BLACK),
            ui_enabled: order.iter().any(|p| p.kind == PassKind::Ui),
        }
    }

    fn render_batches(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        batches: &[DrawBatch],
        clear: ClearColor,
        clear_enabled: bool,
    ) {
        // Pack all vertices / indices into the GPU buffers.
        // Reuse the staging Vecs allocated on the renderer to avoid per-frame allocation.
        self.staging_vertices.clear();
        self.staging_indices.clear();
        self.staging_batch_ranges.clear();

        for batch in batches {
            let v_base = self.staging_vertices.len() as u32;
            let i_start = self.staging_indices.len() as u32;
            self.staging_vertices.extend_from_slice(&batch.vertices);
            for &idx in &batch.indices {
                self.staging_indices.push(idx + v_base);
            }
            let i_end = self.staging_indices.len() as u32;
            self.staging_batch_ranges.push((v_base, i_start, i_end));
        }

        let load_op = if clear_enabled {
            wgpu::LoadOp::Clear(wgpu::Color {
                r: clear.r,
                g: clear.g,
                b: clear.b,
                a: clear.a,
            })
        } else {
            wgpu::LoadOp::Load
        };

        if self.staging_vertices.is_empty() {
            // Nothing to draw — issue a pass only to (optionally) clear.
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear-only"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: load_op,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            return;
        }

        // Upload — compute sizes before borrowing slices so ensure_* can take &mut self.
        let vbytes_len = self.staging_vertices.len() * std::mem::size_of::<Vertex>();
        let ibytes_len = self.staging_indices.len() * std::mem::size_of::<u32>();
        self.ensure_vertex_buffer(vbytes_len as u64);
        self.ensure_index_buffer(ibytes_len as u64);
        let vbytes = bytemuck::cast_slice(&self.staging_vertices);
        let ibytes = bytemuck::cast_slice::<u32, u8>(&self.staging_indices);
        self.ctx.queue.write_buffer(&self.vertex_buffer, 0, vbytes);
        self.ctx.queue.write_buffer(&self.index_buffer, 0, ibytes);

        // Record a single render pass.
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ui-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: load_op,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.globals_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        for (_, i_start, i_end) in &self.staging_batch_ranges {
            pass.draw_indexed(*i_start..*i_end, 0, 0..1);
        }
    }
}

impl Renderer for WgpuRenderer {
    fn resize(&mut self, width: u32, height: u32) {
        let w = width.max(1);
        let h = height.max(1);
        self.ctx.config.width = w;
        self.ctx.config.height = h;
        self.ctx
            .surface
            .configure(&self.ctx.device, &self.ctx.config);

        // Update globals.
        let globals = Globals {
            viewport_size: [w as f32, h as f32],
            _pad: [0.0; 2],
        };
        self.ctx
            .queue
            .write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));
        log::debug!("[wgpu] surface resized to {}x{}", w, h);
    }

    fn render(&mut self, draw_list: &DrawList) -> RenderResult<()> {
        // Batch the draw list.
        let batches: Vec<DrawBatch> = self.batcher.process(draw_list).to_vec();

        // Acquire the next swap-chain frame.
        let output = match self.ctx.surface.get_current_texture() {
            Ok(tex) => tex,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                return Err(RenderError::SurfaceLost);
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return Err(RenderError::Other("out of memory".to_string()));
            }
            Err(wgpu::SurfaceError::Timeout) => {
                return Err(RenderError::Timeout);
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame-encoder"),
            });

        // Resolve enabled passes from the render graph.
        let frame = self.resolve_graph();

        let effective_batches: &[DrawBatch] = if frame.ui_enabled { &batches } else { &[] };
        self.render_batches(
            &mut encoder,
            &view,
            effective_batches,
            frame.clear_color,
            frame.clear_enabled,
        );

        self.ctx.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    fn surface_size(&self) -> (u32, u32) {
        (self.ctx.config.width, self.ctx.config.height)
    }
}

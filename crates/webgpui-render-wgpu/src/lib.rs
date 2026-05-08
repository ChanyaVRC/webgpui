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
use webgpui_render::{DrawCommand, DrawList, RenderError, RenderResult, Renderer};
#[cfg(feature = "filters")]
use webgpui_render_graph::FilterKind;
use webgpui_render_graph::{ClearColor, PassKind, RenderGraph};

// ---------------------------------------------------------------------------
// Filter WGSL shaders (filters feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "filters")]
const BLUR_SHADER_SRC: &str = r#"
struct Uniforms { viewport: vec2<f32>, radius: f32, _pad: f32 };
@group(0) @binding(0) var t_src: texture_2d<f32>;
@group(0) @binding(1) var s_src: sampler;
@group(0) @binding(2) var<uniform> u: Uniforms;
@vertex fn vs_fullscreen(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(pos[vi], 0.0, 1.0);
}
@fragment fn fs_blur(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = p.xy / u.viewport;
    var color = vec4<f32>(0.0);
    var w_sum = 0.0;
    let r = i32(u.radius);
    let sigma = max(u.radius / 3.0, 0.001);
    for (var dy = -r; dy <= r; dy += 1) {
        for (var dx = -r; dx <= r; dx += 1) {
            let offset = vec2<f32>(f32(dx), f32(dy)) / u.viewport;
            let d2 = f32(dx * dx + dy * dy);
            let w = exp(-d2 / (2.0 * sigma * sigma));
            color += textureSample(t_src, s_src, clamp(uv + offset, vec2(0.0), vec2(1.0))) * w;
            w_sum += w;
        }
    }
    return color / w_sum;
}
"#;

#[cfg(feature = "filters")]
const COLOR_MATRIX_SHADER_SRC: &str = r#"
struct Uniforms {
    row0: vec4<f32>, row1: vec4<f32>, row2: vec4<f32>, row3: vec4<f32>,
    offset: vec4<f32>, viewport: vec2<f32>, _pad: vec2<f32>,
};
@group(0) @binding(0) var t_src: texture_2d<f32>;
@group(0) @binding(1) var s_src: sampler;
@group(0) @binding(2) var<uniform> u: Uniforms;
@vertex fn vs_fullscreen(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(pos[vi], 0.0, 1.0);
}
@fragment fn fs_color_matrix(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = p.xy / u.viewport;
    let c = textureSample(t_src, s_src, uv);
    return clamp(vec4<f32>(
        dot(u.row0, c) + u.offset.r,
        dot(u.row1, c) + u.offset.g,
        dot(u.row2, c) + u.offset.b,
        dot(u.row3, c) + u.offset.a,
    ), vec4(0.0), vec4(1.0));
}
"#;

// ---------------------------------------------------------------------------
// Filter uniform structs (filters feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "filters")]
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct BlurUniforms {
    viewport: [f32; 2],
    radius: f32,
    _pad: f32,
}

#[cfg(feature = "filters")]
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct ColorMatrixUniforms {
    rows: [[f32; 4]; 4],
    offset: [f32; 4],
    viewport: [f32; 2],
    _pad: [f32; 2],
}

// ---------------------------------------------------------------------------
// WGSL shader (embedded)
// ---------------------------------------------------------------------------

const IMAGE_SHADER_SRC: &str = r#"
struct Globals {
    viewport_size: vec2<f32>,
    _pad: vec2<f32>,
};
@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var t_image: texture_2d<f32>;
@group(1) @binding(1) var s_image: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv:       vec2<f32>,
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};
@vertex fn vs_image(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let ndc_x =  (in.position.x / globals.viewport_size.x) * 2.0 - 1.0;
    let ndc_y = -(in.position.y / globals.viewport_size.y) * 2.0 + 1.0;
    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}
@fragment fn fs_image(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_image, s_image, in.uv);
}
"#;

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
// Image pipeline types
// ---------------------------------------------------------------------------

/// Per-vertex data for textured quads (position + UV).
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct ImageVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

/// Cached GPU resources for one uploaded image.
struct TextureEntry {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

/// Raw pixel data queued for GPU upload.
pub struct PendingImage {
    /// Unique image ID.
    pub id: u32,
    /// Raw RGBA8 pixel data (row-major, top-to-bottom).
    pub pixels: Vec<u8>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

// ---------------------------------------------------------------------------
// ResolvedFrame – result of querying the RenderGraph for a frame
// ---------------------------------------------------------------------------

struct ResolvedFrame {
    clear_enabled: bool,
    clear_color: ClearColor,
    ui_enabled: bool,
    #[cfg(feature = "filters")]
    filter_kinds: Vec<FilterKind>,
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
    staging_batch_ranges: Vec<(u32, u32)>,
    // ---- Image pipeline ----
    image_pipeline: wgpu::RenderPipeline,
    image_bgl: wgpu::BindGroupLayout,
    image_sampler: wgpu::Sampler,
    /// Texture cache keyed by image ID.
    texture_cache: std::collections::HashMap<u32, TextureEntry>,
    image_vertex_buffer: wgpu::Buffer,
    image_vertex_capacity: u64,
    image_index_buffer: wgpu::Buffer,
    image_index_capacity: u64,
    image_staging_verts: Vec<ImageVertex>,
    image_staging_idx: Vec<u32>,
    // -- filter pipeline (filters feature) -----------------------------------
    #[cfg(feature = "filters")]
    filter_bgl: wgpu::BindGroupLayout,
    #[cfg(feature = "filters")]
    blur_pipeline: wgpu::RenderPipeline,
    #[cfg(feature = "filters")]
    color_matrix_pipeline: wgpu::RenderPipeline,
    #[cfg(feature = "filters")]
    filter_uniform_buffer: wgpu::Buffer,
    #[cfg(feature = "filters")]
    filter_sampler: wgpu::Sampler,
    #[cfg(feature = "filters")]
    filter_texture: Option<wgpu::Texture>,
    #[cfg(feature = "filters")]
    filter_texture_view: Option<wgpu::TextureView>,
    #[cfg(feature = "filters")]
    filter_bind_group: Option<wgpu::BindGroup>,
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

        // ---- Image pipeline setup ----
        let image_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("image-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let image_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("image-shader"),
            source: wgpu::ShaderSource::Wgsl(IMAGE_SHADER_SRC.into()),
        });
        let image_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("image-pipeline-layout"),
                bind_group_layouts: &[&globals_bgl, &image_bgl],
                push_constant_ranges: &[],
            });
        let image_vbl = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ImageVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        };
        let image_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("image-pipeline"),
            layout: Some(&image_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &image_shader,
                entry_point: "vs_image",
                buffers: &[image_vbl],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &image_shader,
                entry_point: "fs_image",
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
        let image_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("image-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        const INITIAL_IMG_BUF: u64 = 64 * 1024;
        let image_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image-vbuf"),
            size: INITIAL_IMG_BUF,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let image_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image-ibuf"),
            size: INITIAL_IMG_BUF,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ---- filter pipeline (filters feature) ----
        #[cfg(feature = "filters")]
        let filter_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("filter-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        #[cfg(feature = "filters")]
        let filter_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("filter-pipeline-layout"),
                bind_group_layouts: &[&filter_bgl],
                push_constant_ranges: &[],
            });
        #[cfg(feature = "filters")]
        let blur_pipeline = {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("blur-shader"),
                source: wgpu::ShaderSource::Wgsl(BLUR_SHADER_SRC.into()),
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("blur-pipeline"),
                layout: Some(&filter_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_fullscreen",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_blur",
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: ctx.surface_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            })
        };
        #[cfg(feature = "filters")]
        let color_matrix_pipeline = {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("color-matrix-shader"),
                source: wgpu::ShaderSource::Wgsl(COLOR_MATRIX_SHADER_SRC.into()),
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("color-matrix-pipeline"),
                layout: Some(&filter_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_fullscreen",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_color_matrix",
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: ctx.surface_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            })
        };
        #[cfg(feature = "filters")]
        let filter_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("filter-uniform"),
            size: 128,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        #[cfg(feature = "filters")]
        let filter_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("filter-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
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
            image_pipeline,
            image_bgl,
            image_sampler,
            texture_cache: std::collections::HashMap::new(),
            image_vertex_buffer,
            image_vertex_capacity: INITIAL_IMG_BUF,
            image_index_buffer,
            image_index_capacity: INITIAL_IMG_BUF,
            image_staging_verts: Vec::new(),
            image_staging_idx: Vec::new(),
            #[cfg(feature = "filters")]
            filter_bgl,
            #[cfg(feature = "filters")]
            blur_pipeline,
            #[cfg(feature = "filters")]
            color_matrix_pipeline,
            #[cfg(feature = "filters")]
            filter_uniform_buffer,
            #[cfg(feature = "filters")]
            filter_sampler,
            #[cfg(feature = "filters")]
            filter_texture: None,
            #[cfg(feature = "filters")]
            filter_texture_view: None,
            #[cfg(feature = "filters")]
            filter_bind_group: None,
        }
    }

    /// Uploads pending images to GPU textures and caches them by ID.
    ///
    /// Call once per frame before [`Renderer::render`] to materialise any
    /// images loaded via [`DrawContext::load_image`].
    pub fn upload_images(&mut self, pending: Vec<PendingImage>) {
        for img in pending {
            if self.texture_cache.contains_key(&img.id) {
                continue;
            }
            let texture = self.ctx.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("image-tex"),
                size: wgpu::Extent3d {
                    width: img.width,
                    height: img.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.ctx.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &img.pixels,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * img.width),
                    rows_per_image: Some(img.height),
                },
                wgpu::Extent3d {
                    width: img.width,
                    height: img.height,
                    depth_or_array_layers: 1,
                },
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = self
                .ctx
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("image-bg"),
                    layout: &self.image_bgl,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.image_sampler),
                        },
                    ],
                });
            self.texture_cache.insert(
                img.id,
                TextureEntry {
                    texture,
                    bind_group,
                },
            );
            log::debug!(
                "[wgpu] uploaded image id={} ({}x{})",
                img.id,
                img.width,
                img.height
            );
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
            let new_size = needed.next_power_of_two();
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

    fn ensure_image_vertex_buffer(&mut self, needed: u64) {
        if needed > self.image_vertex_capacity {
            let new_size = needed.next_power_of_two();
            self.image_vertex_buffer = self.ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("image-vbuf"),
                size: new_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.image_vertex_capacity = new_size;
        }
    }

    fn ensure_image_index_buffer(&mut self, needed: u64) {
        if needed > self.image_index_capacity {
            let new_size = needed.next_power_of_two();
            self.image_index_buffer = self.ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("image-ibuf"),
                size: new_size,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.image_index_capacity = new_size;
        }
    }

    /// Renders all `DrawImage` commands from `draw_list` into `view` (after the color pass).
    fn render_images(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        draw_list: &DrawList,
    ) {
        // Collect image draw commands in order.
        let image_cmds: Vec<_> = draw_list
            .commands()
            .iter()
            .filter_map(|cmd| {
                if let DrawCommand::DrawImage { rect, handle, .. } = cmd {
                    Some((*rect, handle.0))
                } else {
                    None
                }
            })
            .collect();
        if image_cmds.is_empty() {
            return;
        }

        // Build one quad per image command (UVs cover the full texture).
        self.image_staging_verts.clear();
        self.image_staging_idx.clear();

        // We render each image as a separate draw call (one bind group per texture).
        // Build per-command ranges: (image_id, index_start, index_end).
        let mut ranges: Vec<(u32, u32, u32)> = Vec::new();

        for (rect, image_id) in &image_cmds {
            if !self.texture_cache.contains_key(image_id) {
                continue;
            }
            let base = self.image_staging_verts.len() as u32;
            let (x0, y0, x1, y1) = (rect.min_x(), rect.min_y(), rect.max_x(), rect.max_y());
            self.image_staging_verts.extend_from_slice(&[
                ImageVertex {
                    position: [x0, y0],
                    uv: [0.0, 0.0],
                },
                ImageVertex {
                    position: [x1, y0],
                    uv: [1.0, 0.0],
                },
                ImageVertex {
                    position: [x1, y1],
                    uv: [1.0, 1.0],
                },
                ImageVertex {
                    position: [x0, y1],
                    uv: [0.0, 1.0],
                },
            ]);
            let i_start = self.image_staging_idx.len() as u32;
            self.image_staging_idx.extend_from_slice(&[
                base,
                base + 1,
                base + 2,
                base + 2,
                base + 3,
                base,
            ]);
            let i_end = self.image_staging_idx.len() as u32;
            ranges.push((*image_id, i_start, i_end));
        }

        if self.image_staging_verts.is_empty() {
            return;
        }

        // Compute byte sizes before mutably borrowing self for buffer ensures.
        let vlen = (self.image_staging_verts.len() * std::mem::size_of::<ImageVertex>()) as u64;
        let ilen = (self.image_staging_idx.len() * std::mem::size_of::<u32>()) as u64;
        self.ensure_image_vertex_buffer(vlen);
        self.ensure_image_index_buffer(ilen);
        // Upload to GPU buffers — casts happen after ensure calls have returned.
        let vbytes = bytemuck::cast_slice::<ImageVertex, u8>(&self.image_staging_verts);
        self.ctx
            .queue
            .write_buffer(&self.image_vertex_buffer, 0, vbytes);
        let ibytes = bytemuck::cast_slice::<u32, u8>(&self.image_staging_idx);
        self.ctx
            .queue
            .write_buffer(&self.image_index_buffer, 0, ibytes);

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("image-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.image_pipeline);
        pass.set_bind_group(0, &self.globals_bind_group, &[]);
        pass.set_vertex_buffer(0, self.image_vertex_buffer.slice(..));
        pass.set_index_buffer(self.image_index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        for (image_id, i_start, i_end) in ranges {
            if let Some(entry) = self.texture_cache.get(&image_id) {
                pass.set_bind_group(1, &entry.bind_group, &[]);
                pass.draw_indexed(i_start..i_end, 0, 0..1);
            }
        }
    }

    #[cfg(feature = "filters")]
    fn ensure_filter_texture(&mut self, width: u32, height: u32) {
        if let Some(ref tex) = self.filter_texture {
            let size = tex.size();
            if size.width == width && size.height == height {
                return;
            }
        }
        let texture = self.ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("filter-tex"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.ctx.surface_format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self
            .ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("filter-bg"),
                layout: &self.filter_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.filter_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.filter_uniform_buffer.as_entire_binding(),
                    },
                ],
            });
        self.filter_texture = Some(texture);
        self.filter_texture_view = Some(view);
        self.filter_bind_group = Some(bind_group);
    }

    #[cfg(feature = "filters")]
    fn apply_filters(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        filter_kinds: &[FilterKind],
        surface_view: &wgpu::TextureView,
    ) {
        let w = self.ctx.config.width as f32;
        let h = self.ctx.config.height as f32;
        for kind in filter_kinds {
            match kind {
                FilterKind::Blur(params) => {
                    let u = BlurUniforms {
                        viewport: [w, h],
                        radius: params.radius,
                        _pad: 0.0,
                    };
                    self.ctx.queue.write_buffer(
                        &self.filter_uniform_buffer,
                        0,
                        bytemuck::bytes_of(&u),
                    );
                }
                FilterKind::ColorMatrix(params) => {
                    let m = &params.matrix;
                    let u = ColorMatrixUniforms {
                        rows: [
                            [m[0], m[1], m[2], m[3]],
                            [m[5], m[6], m[7], m[8]],
                            [m[10], m[11], m[12], m[13]],
                            [m[15], m[16], m[17], m[18]],
                        ],
                        offset: [m[4], m[9], m[14], m[19]],
                        viewport: [w, h],
                        _pad: [0.0; 2],
                    };
                    self.ctx.queue.write_buffer(
                        &self.filter_uniform_buffer,
                        0,
                        bytemuck::bytes_of(&u),
                    );
                }
            }
            let pipeline = match kind {
                FilterKind::Blur(_) => &self.blur_pipeline,
                FilterKind::ColorMatrix(_) => &self.color_matrix_pipeline,
            };
            let bg = self
                .filter_bind_group
                .as_ref()
                .expect("ensure_filter_texture must be called before apply_filters");
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("filter-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: surface_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, bg, &[]);
                pass.draw(0..3, 0..1);
            }
        }
    }

    fn ensure_index_buffer(&mut self, needed: u64) {
        if needed > self.index_buffer_capacity {
            let new_size = needed.next_power_of_two();
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
        #[cfg(feature = "filters")]
        let filter_kinds: Vec<FilterKind> = order
            .iter()
            .filter_map(|p| {
                if let PassKind::Filter(kind) = p.kind {
                    Some(kind)
                } else {
                    None
                }
            })
            .collect();
        ResolvedFrame {
            clear_enabled: order.iter().any(|p| p.kind == PassKind::Clear),
            clear_color: order
                .iter()
                .find(|p| p.kind == PassKind::Clear)
                .map(|p| p.clear_color)
                .unwrap_or(ClearColor::BLACK),
            ui_enabled: order.iter().any(|p| p.kind == PassKind::Ui),
            #[cfg(feature = "filters")]
            filter_kinds,
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
            self.staging_batch_ranges.push((i_start, i_end));
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

        let sw = self.ctx.config.width;
        let sh = self.ctx.config.height;
        for ((i_start, i_end), batch) in self.staging_batch_ranges.iter().zip(batches.iter()) {
            if let Some(sci) = batch.scissor {
                // Convert logical pixel rect to physical pixels (assume scale=1 for MVP).
                // Clamp to surface bounds to avoid wgpu validation errors.
                let sx = (sci.min_x() as u32).min(sw);
                let sy = (sci.min_y() as u32).min(sh);
                let sw2 = ((sci.max_x() as u32).min(sw)).saturating_sub(sx);
                let sh2 = ((sci.max_y() as u32).min(sh)).saturating_sub(sy);
                pass.set_scissor_rect(sx, sy, sw2, sh2);
            } else {
                pass.set_scissor_rect(0, 0, sw, sh);
            }
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

        // Drop filter resources so ensure_filter_texture recreates them at the new size.
        #[cfg(feature = "filters")]
        {
            self.filter_texture = None;
            self.filter_texture_view = None;
            self.filter_bind_group = None;
        }
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

        // When filters are active, UI renders into an offscreen texture; the
        // filter pass then composites it to the surface.
        #[cfg(feature = "filters")]
        let filter_view: Option<wgpu::TextureView> = if frame.filter_kinds.is_empty() {
            None
        } else {
            let (w, h) = (self.ctx.config.width, self.ctx.config.height);
            self.ensure_filter_texture(w, h);
            Some(
                self.filter_texture
                    .as_ref()
                    .unwrap()
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            )
        };

        #[cfg(feature = "filters")]
        let ui_target: &wgpu::TextureView = if let Some(ref fv) = filter_view {
            fv
        } else {
            &view
        };
        #[cfg(not(feature = "filters"))]
        let ui_target: &wgpu::TextureView = &view;

        let effective_batches: &[DrawBatch] = if frame.ui_enabled { &batches } else { &[] };
        self.render_batches(
            &mut encoder,
            ui_target,
            effective_batches,
            frame.clear_color,
            frame.clear_enabled,
        );

        // Render images on top of the color geometry pass (same target as UI).
        self.render_images(&mut encoder, ui_target, draw_list);

        // Apply filter passes (reads from offscreen texture, writes to surface).
        #[cfg(feature = "filters")]
        if !frame.filter_kinds.is_empty() {
            self.apply_filters(&mut encoder, &frame.filter_kinds, &view);
        }

        self.ctx.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    fn surface_size(&self) -> (u32, u32) {
        (self.ctx.config.width, self.ctx.config.height)
    }
}

// ---------------------------------------------------------------------------
// Tests — GPU-independent
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_image_fields_preserved() {
        let img = PendingImage {
            id: 42,
            pixels: vec![255u8; 4 * 4 * 4],
            width: 4,
            height: 4,
        };
        assert_eq!(img.id, 42);
        assert_eq!(img.width, 4);
        assert_eq!(img.height, 4);
        assert_eq!(img.pixels.len(), 64);
    }

    #[test]
    fn image_vertex_size_and_bytemuck() {
        // position: [f32; 2] + uv: [f32; 2] = 16 bytes, align 4
        assert_eq!(std::mem::size_of::<ImageVertex>(), 16);
        assert_eq!(std::mem::align_of::<ImageVertex>(), 4);
        let v = ImageVertex {
            position: [1.0, 2.0],
            uv: [0.5, 0.75],
        };
        let bytes: &[u8] = bytemuck::bytes_of(&v);
        assert_eq!(bytes.len(), 16);
    }

    #[cfg(feature = "filters")]
    mod filter_tests {
        use super::*;
        use webgpui_render_graph::{BlurParams, ColorMatrixParams, FilterKind};

        #[test]
        fn blur_uniforms_size() {
            // viewport [f32;2] + radius f32 + _pad f32 = 16 bytes
            assert_eq!(std::mem::size_of::<BlurUniforms>(), 16);
            assert_eq!(std::mem::align_of::<BlurUniforms>(), 4);
        }

        #[test]
        fn color_matrix_uniforms_size() {
            // rows [[f32;4];4] = 64B, offset [f32;4] = 16B, viewport [f32;2] = 8B, _pad [f32;2] = 8B
            assert_eq!(std::mem::size_of::<ColorMatrixUniforms>(), 96);
        }

        #[test]
        fn blur_params_round_trip() {
            let kind = FilterKind::Blur(BlurParams { radius: 5.0 });
            if let FilterKind::Blur(p) = kind {
                assert_eq!(p.radius, 5.0);
            } else {
                panic!("wrong variant");
            }
        }

        #[test]
        fn color_matrix_identity_diagonal() {
            let m = ColorMatrixParams::IDENTITY.matrix;
            assert_eq!(m[0], 1.0); // R→R
            assert_eq!(m[6], 1.0); // G→G
            assert_eq!(m[12], 1.0); // B→B
            assert_eq!(m[18], 1.0); // A→A
            assert_eq!(m[1], 0.0); // R→G off-diag
            assert_eq!(m[4], 0.0); // R offset
        }

        #[test]
        fn color_matrix_uniforms_round_trip() {
            let m = ColorMatrixParams::GRAYSCALE.matrix;
            let u = ColorMatrixUniforms {
                rows: [
                    [m[0], m[1], m[2], m[3]],
                    [m[5], m[6], m[7], m[8]],
                    [m[10], m[11], m[12], m[13]],
                    [m[15], m[16], m[17], m[18]],
                ],
                offset: [m[4], m[9], m[14], m[19]],
                viewport: [800.0, 600.0],
                _pad: [0.0; 2],
            };
            // Grayscale: R-row weights sum to ~1.0
            let sum: f32 = u.rows[0].iter().sum();
            assert!(
                (sum - 1.0).abs() < 1e-5,
                "R row weights should sum to 1.0, got {}",
                sum
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Tests — GPU-required (cargo test --features test-gpu)
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "test-gpu"))]
mod gpu_tests {
    use super::*;

    struct HeadlessGpu {
        device: wgpu::Device,
        queue: wgpu::Queue,
    }

    /// Returns `None` when no adapter is available (headless CI without GPU).
    fn init_headless() -> Option<HeadlessGpu> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::None,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        ))
        .ok()?;
        Some(HeadlessGpu { device, queue })
    }

    #[test]
    fn image_shader_compiles() {
        let Some(gpu) = init_headless() else {
            return;
        };
        // Panics if WGSL is syntactically or semantically invalid.
        let _ = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("image-shader-test"),
                source: wgpu::ShaderSource::Wgsl(IMAGE_SHADER_SRC.into()),
            });
    }

    #[test]
    fn texture_upload_rgba8_succeeds() {
        let Some(gpu) = init_headless() else {
            return;
        };
        let (w, h) = (8u32, 8u32);
        let pixels = vec![128u8; (4 * w * h) as usize];
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test-tex"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        gpu.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit([]);
    }

    #[test]
    #[cfg(feature = "filters")]
    fn blur_shader_compiles() {
        let Some(gpu) = init_headless() else {
            return;
        };
        let _ = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("blur-shader-test"),
                source: wgpu::ShaderSource::Wgsl(BLUR_SHADER_SRC.into()),
            });
    }

    #[test]
    #[cfg(feature = "filters")]
    fn color_matrix_shader_compiles() {
        let Some(gpu) = init_headless() else {
            return;
        };
        let _ = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("cm-shader-test"),
                source: wgpu::ShaderSource::Wgsl(COLOR_MATRIX_SHADER_SRC.into()),
            });
    }

    #[test]
    fn image_bind_group_layout_accepted() {
        let Some(gpu) = init_headless() else {
            return;
        };
        let _bgl = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
    }
}

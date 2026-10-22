//! The wgpu side of text: one pipeline drawing atlas quads with the
//! gamma and contrast shader, the atlas texture it samples, and the
//! buffers it draws from. Create once per target format.

use crate::GlyphAtlas;

/// The two shader parameters. See `docs/text.md` for how they were chosen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlendParams {
    /// Exponent taking encoded channels to linear light.
    pub gamma: f32,
    /// Coverage boost for dark text on light backgrounds; 0 is none.
    pub contrast: f32,
}

impl Default for BlendParams {
    /// Chromium's Skia build on Linux uses 1.2 and 0.2; the comparison
    /// page in `docs/text.md` is where these were confirmed or moved.
    fn default() -> Self {
        Self {
            gamma: 1.2,
            contrast: 0.2,
        }
    }
}

/// Floats per vertex: position 2, uv 2, colour 4.
pub(crate) const FLOATS_PER_VERTEX: usize = 8;
const VERTEX_SIZE: u64 = (FLOATS_PER_VERTEX * std::mem::size_of::<f32>()) as u64;

/// Draws laid-out text into a render target.
pub struct TextPipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(crate) sampler: wgpu::Sampler,
    pub(crate) uniforms: wgpu::Buffer,
    pub(crate) texture: wgpu::Texture,
    pub(crate) vertices: Option<wgpu::Buffer>,
}

impl TextPipeline {
    /// A pipeline for targets of `format`, with an atlas texture matching
    /// `atlas`'s coverage page.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, atlas: &GlyphAtlas) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text"),
            source: wgpu::ShaderSource::Wgsl(include_str!("text.wgsl").into()),
        });
        let layout = bind_group_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        Self {
            pipeline: render_pipeline(device, &pipeline_layout, &shader, format),
            layout,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("text nearest"),
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }),
            uniforms: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("text params"),
                size: 16,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            texture: coverage_texture(device, atlas.coverage_page().size()),
            vertices: None,
        }
    }

    /// Make sure the vertex buffer holds at least `bytes`; it is reused
    /// across frames and only grows.
    pub(crate) fn reserve_vertices(&mut self, device: &wgpu::Device, bytes: u64) {
        if self.vertices.as_ref().is_some_and(|b| b.size() >= bytes) {
            return;
        }
        self.vertices = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text quads"),
            size: bytes.max(VERTEX_SIZE * 6 * 64),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }

    pub(crate) fn bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        let view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("text"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniforms.as_entire_binding(),
                },
            ],
        })
    }
}

fn coverage_texture(device: &wgpu::Device, size: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("glyph atlas coverage"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("text"),
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
    })
}

fn render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("text"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: VERTEX_SIZE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
            })],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

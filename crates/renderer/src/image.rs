//! Drawing a texture into a viewport under a [`Transform`]: the canvas
//! surface for an image. Nearest sampling, so at any zoom the pixel
//! under a viewport point is exactly one image pixel.

use vapors_scene::{Size, Transform};

/// A pipeline that draws one 2D texture as a quad. Create once per
/// target format; draw many times.
pub struct ImagePipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    vertices: wgpu::Buffer,
}

/// One vertex: viewport position in clip space, texture coordinate.
#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
}

const VERTEX_COUNT: u32 = 6;
const VERTEX_SIZE: u64 = (std::mem::size_of::<Vertex>()) as u64;

impl ImagePipeline {
    /// Build the pipeline for targets of `format`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("image"),
            source: wgpu::ShaderSource::Wgsl(include_str!("image.wgsl").into()),
        });
        let layout = bind_group_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("image"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = render_pipeline(device, &pipeline_layout, &shader, format);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("image nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image quad"),
            size: VERTEX_SIZE * u64::from(VERTEX_COUNT),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            layout,
            sampler,
            vertices,
        }
    }

    /// Draw `texture` into `target` (of size `viewport`) placed by `transform`.
    /// Pixels outside the image keep whatever was in the target.
    pub fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        viewport: Size,
        texture: &wgpu::Texture,
        transform: Transform,
    ) {
        let image = Size {
            width: texture.width(),
            height: texture.height(),
        };
        let quad = quad_vertices(transform.image_bounds(image), viewport);
        queue.write_buffer(&self.vertices, 0, &as_bytes(&quad));
        let bind_group = self.bind_group(device, texture);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("image"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("image"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertices.slice(..));
            pass.draw(0..VERTEX_COUNT, 0..1);
        }
        queue.submit([encoder.finish()]);
    }

    fn bind_group(&self, device: &wgpu::Device, texture: &wgpu::Texture) -> wgpu::BindGroup {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image"),
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
            ],
        })
    }
}

fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("image"),
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
    })
}

fn render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("image"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: VERTEX_SIZE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
            })],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

/// Two triangles covering the image's viewport rectangle, in clip space.
fn quad_vertices(bounds: [[f32; 2]; 2], viewport: Size) -> [Vertex; VERTEX_COUNT as usize] {
    let [[left, top], [right, bottom]] = bounds;
    let corner = |x: f32, y: f32, u: f32, v: f32| Vertex {
        position: to_clip([x, y], viewport),
        uv: [u, v],
    };
    [
        corner(left, top, 0.0, 0.0),
        corner(right, top, 1.0, 0.0),
        corner(left, bottom, 0.0, 1.0),
        corner(right, top, 1.0, 0.0),
        corner(right, bottom, 1.0, 1.0),
        corner(left, bottom, 0.0, 1.0),
    ]
}

/// Viewport pixels (origin top-left, y down) to clip space (-1..1, y up).
#[allow(clippy::cast_precision_loss)] // pixel counts are far below 2^24
fn to_clip(point: [f32; 2], viewport: Size) -> [f32; 2] {
    [
        point[0] / viewport.width as f32 * 2.0 - 1.0,
        1.0 - point[1] / viewport.height as f32 * 2.0,
    ]
}

fn as_bytes(vertices: &[Vertex; VERTEX_COUNT as usize]) -> Vec<u8> {
    vertices
        .iter()
        .flat_map(|v| [v.position[0], v.position[1], v.uv[0], v.uv[1]])
        .flat_map(f32::to_le_bytes)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_corners_map_to_clip_corners() {
        // Given
        let viewport = Size {
            width: 640,
            height: 400,
        };

        // When
        let top_left = to_clip([0.0, 0.0], viewport);
        let bottom_right = to_clip([640.0, 400.0], viewport);

        // Then
        assert_eq!(top_left, [-1.0, 1.0]);
        assert_eq!(bottom_right, [1.0, -1.0]);
    }

    #[test]
    fn quad_covers_the_bounds_with_matching_texture_coordinates() {
        // Given
        let viewport = Size {
            width: 100,
            height: 100,
        };

        // When
        let quad = quad_vertices([[0.0, 0.0], [50.0, 100.0]], viewport);

        // Then: first vertex is the top-left at uv 0,0; last is bottom-left at uv 0,1
        assert_eq!(quad[0].position, [-1.0, 1.0]);
        assert_eq!(quad[0].uv, [0.0, 0.0]);
        assert_eq!(quad[4].position, [0.0, -1.0]);
        assert_eq!(quad[4].uv, [1.0, 1.0]);
    }
}

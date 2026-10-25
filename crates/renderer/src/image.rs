//! Drawing a texture into a viewport under a [`Transform`]: the canvas
//! surface for an image, with nearest sampling, so at any zoom the pixel
//! under a viewport point is exactly one image pixel. And stretched over
//! a rect inside the target's scissor, sampled smoothly: an `img`.

use vapors_scene::{Rect, Size, Transform};

use crate::Target;
use crate::image_setup::{bind_group_layout, render_pipeline};

use crate::quad::{VERTEX_COUNT, VERTEX_SIZE, quad_vertices, vertex_bytes};

/// A pipeline that draws one 2D texture as a quad. Create once per
/// target format; draw many times.
pub struct ImagePipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    /// Nearest sampling, for the canvas: one image pixel per pixel.
    nearest: wgpu::Sampler,
    /// Linear sampling, for an `img` stretched over its box.
    smooth: wgpu::Sampler,
    vertices: wgpu::Buffer,
}

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
        let sampler = |label, filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        let (nearest, smooth) = (
            sampler("image nearest", wgpu::FilterMode::Nearest),
            sampler("image smooth", wgpu::FilterMode::Linear),
        );
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image quad"),
            size: VERTEX_SIZE * u64::from(VERTEX_COUNT),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            layout,
            nearest,
            smooth,
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
        let at = Target {
            device,
            queue,
            view: target,
            viewport,
            scissor: None,
        };
        self.quad(&at, texture, transform.image_bounds(image), &self.nearest);
    }

    /// Draw `texture` stretched over `rect` (device pixels) in `target`,
    /// sampled smoothly, inside the target's scissor: how an `img` is
    /// drawn.
    pub fn draw_rect(&self, target: &Target<'_>, texture: &wgpu::Texture, rect: Rect) {
        let bounds = [[rect.left, rect.top], [rect.right, rect.bottom]];
        self.quad(target, texture, bounds, &self.smooth);
    }

    fn quad(
        &self,
        target: &Target<'_>,
        texture: &wgpu::Texture,
        bounds: [[f32; 2]; 2],
        sampler: &wgpu::Sampler,
    ) {
        let quad = quad_vertices(bounds, target.viewport);
        target
            .queue
            .write_buffer(&self.vertices, 0, &vertex_bytes(&quad));
        let bind_group = self.bind_group(target.device, texture, sampler);
        let mut encoder = target
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("image"),
            });
        {
            let mut pass = crate::quads_setup::load_pass(&mut encoder, target.view);
            if let Some(s) = target.scissor {
                pass.set_scissor_rect(s.x, s.y, s.width, s.height);
            }
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertices.slice(..));
            pass.draw(0..VERTEX_COUNT, 0..1);
        }
        target.queue.submit([encoder.finish()]);
    }

    fn bind_group(
        &self,
        device: &wgpu::Device,
        texture: &wgpu::Texture,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
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
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }
}

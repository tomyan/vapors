//! Drawing a texture into a viewport under a [`Transform`]: the canvas
//! surface for an image, with nearest sampling, so at any zoom the pixel
//! under a viewport point is exactly one image pixel. And stretched over
//! a rect inside the target's scissor, sampled smoothly: an `img`, bound
//! once ([`TextureBinding`]) and drawn each frame through that binding.

use vapors_scene::{Rect, Size, Transform};

use crate::frame_arena::FrameArena;
use crate::image_binding::{Sampling, TextureBinding};
use crate::image_setup::{bind_group_layout, render_pipeline};
use crate::{ArenaFull, FrameEncoder, GpuWork, Target, WorkCounter};

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
    vertices: FrameArena,
    work: WorkCounter,
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
        let sampler = |label, filter, mipmap_filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter,
                ..Default::default()
            })
        };
        let (nearest, smooth) = (
            sampler(
                "image nearest",
                wgpu::FilterMode::Nearest,
                wgpu::MipmapFilterMode::Nearest,
            ),
            sampler(
                "image smooth",
                wgpu::FilterMode::Linear,
                wgpu::MipmapFilterMode::Linear,
            ),
        );
        let vertices = FrameArena::new(
            device,
            "image quad",
            wgpu::BufferUsages::VERTEX,
            VERTEX_SIZE,
        );
        Self {
            pipeline,
            layout,
            nearest,
            smooth,
            vertices,
            work: WorkCounter::default(),
        }
    }

    /// Draw `texture` into `target` (of size `viewport`) placed by `transform`.
    /// Pixels outside the image keep whatever was in the target. Submits
    /// its own encoder: the canvas draws one image a frame.
    ///
    /// # Errors
    ///
    /// [`ArenaFull`] when the image's vertices do not fit a buffer of the
    /// device's `max_buffer_size`; nothing is drawn then.
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        viewport: Size,
        texture: &wgpu::Texture,
        transform: Transform,
    ) -> Result<(), ArenaFull> {
        let image = Size {
            width: texture.width(),
            height: texture.height(),
        };
        let encoder = FrameEncoder::new(device);
        let at = Target {
            device,
            queue,
            encoder: &encoder,
            view: target,
            viewport,
            scissor: None,
        };
        let bound = self.bind(device, texture, Sampling::Nearest);
        self.quad(&at, &bound, transform.image_bounds(image));
        encoder.submit(queue)
    }

    /// Binds `texture` for drawing, sampled as `sampling`: one view and
    /// one bind group, counted. Keep the binding beside the texture for
    /// as long as it is drawn, and drop it with the texture.
    #[must_use]
    pub fn bind(
        &self,
        device: &wgpu::Device,
        texture: &wgpu::Texture,
        sampling: Sampling,
    ) -> TextureBinding {
        let _binding = tracing::trace_span!("draw step", phase = "encode").entered();
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = match sampling {
            Sampling::Nearest => &self.nearest,
            Sampling::Smooth => &self.smooth,
        };
        let bind_group = self.bind_group(device, &view, sampler);
        TextureBinding::new(texture.clone(), bind_group)
    }

    /// Draw `bound`'s texture stretched over `rect` (device pixels) in
    /// `target`, inside the target's scissor: how an `img` is drawn.
    pub fn draw_rect(&mut self, target: &Target<'_>, bound: &TextureBinding, rect: Rect) {
        let bounds = [[rect.left, rect.top], [rect.right, rect.bottom]];
        self.quad(target, bound, bounds);
    }

    /// The work its draws issued since the last take; the count restarts.
    pub fn take_work(&self) -> GpuWork {
        self.work.take()
    }

    fn quad(&mut self, target: &Target<'_>, bound: &TextureBinding, bounds: [[f32; 2]; 2]) {
        let quad = vertex_bytes(&quad_vertices(bounds, target.viewport));
        let Some(vertices) = self
            .vertices
            .push(target.encoder, target.queue, &self.work, &quad)
        else {
            return;
        };
        let _encoding = tracing::trace_span!("draw step", phase = "encode").entered();
        target.draw_pass(|pass| {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bound.bind_group(), &[]);
            pass.set_vertex_buffer(0, vertices.slice());
            self.work.draw(pass, 0..VERTEX_COUNT, 0..1);
        });
    }

    fn bind_group(
        &self,
        device: &wgpu::Device,
        view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        self.work.create_bind_group(
            device,
            &wgpu::BindGroupDescriptor {
                label: Some("image"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            },
        )
    }
}

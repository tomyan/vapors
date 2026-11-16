//! A surface's backing texture: what its renderer draws into and the
//! painter composites, bound for the image pipeline when it is made.
//! Another size is another backing, binding and all.

use vapors_renderer::{ImagePipeline, Sampling, Target, TextureBinding, WorkCounter};

use super::super::errors::Refusal;
use crate::surface::{SURFACE_FORMAT, SurfaceError, SurfaceFrame, SurfaceRenderer, SurfaceSize};

/// The texture of one active surface, bound for compositing, and the
/// size it was made at.
pub(super) struct Backing {
    pub(super) bound: TextureBinding,
    pub(super) view: wgpu::TextureView,
    pub(super) size: SurfaceSize,
}

impl Backing {
    pub(super) fn new(device: &wgpu::Device, pipeline: &ImagePipeline, size: SurfaceSize) -> Self {
        let texture = device.create_texture(&descriptor(size));
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            bound: pipeline.bind(device, &texture, Sampling::Smooth),
            view,
            size,
        }
    }

    /// Clears the backing in a submit of its own: the first submit that
    /// uses it, before the first frame (`Mounted::start_at`).
    pub(super) fn clear(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&ENCODER);
        let clear = wgpu::RenderPassColorAttachment {
            view: &self.view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        };
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gpu-surface backing cleared at start"),
            color_attachments: &[Some(clear)],
            ..Default::default()
        }));
        queue.submit([encoder.finish()]);
    }
}

const ENCODER: wgpu::CommandEncoderDescriptor<'static> = wgpu::CommandEncoderDescriptor {
    label: Some("gpu-surface"),
};

/// `renderer`'s render into `backing`, submitted on `target`'s queue.
pub(super) fn render_into(
    renderer: &mut Box<dyn SurfaceRenderer>,
    target: &Target<'_>,
    backing: &Backing,
    work: &WorkCounter,
) -> Result<(), SurfaceError> {
    let mut encoder = target.device.create_command_encoder(&ENCODER);
    // a timed frame's pair around the renderer's passes (GpuTimer)
    target
        .encoder
        .time_section(&mut encoder, &backing.view, |encoder| {
            renderer.render(SurfaceFrame {
                device: target.device,
                queue: target.queue,
                encoder,
                view: &backing.view,
                size: backing.size,
                work,
            })
        })?;
    let commands = encoder.finish();
    let _submit = tracing::trace_span!("draw step", phase = "submit").entered();
    target.encoder.submit_section(target.queue, commands);
    Ok(())
}

/// A backing is rendered into, sampled, and copyable both ways.
const USAGE: wgpu::TextureUsages = wgpu::TextureUsages::RENDER_ATTACHMENT
    .union(wgpu::TextureUsages::TEXTURE_BINDING)
    .union(wgpu::TextureUsages::COPY_SRC)
    .union(wgpu::TextureUsages::COPY_DST);

/// A [`SURFACE_FORMAT`] backing of `size`.
fn descriptor(size: SurfaceSize) -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: Some("gpu-surface backing"),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SURFACE_FORMAT,
        usage: USAGE,
        view_formats: &[],
    }
}

/// Refuses a backing larger than the device's largest 2D texture.
pub(super) fn fits(device: &wgpu::Device, size: SurfaceSize) -> Result<(), Refusal> {
    let limit = device.limits().max_texture_dimension_2d;
    if size.width > limit || size.height > limit {
        return Err(Refusal::TooLarge { size, limit });
    }
    Ok(())
}

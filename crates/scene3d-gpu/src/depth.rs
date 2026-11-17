//! The scene pass's attachments, made for each backing size: the depth
//! buffer, and at 4 samples the multisampled colour the pass draws into
//! and resolves into the backing (`docs/plans/flat-scene-f7.md`, F7b).

use crate::pipeline::DEPTH_FORMAT;
use vapors_app::SURFACE_FORMAT;
use vapors_app::SurfaceSize;
use vapors_renderer::Samples;

/// The depth view, and the multisampled colour view at 4 samples.
pub(crate) struct Attachments {
    pub(crate) depth: wgpu::TextureView,
    pub(crate) colour: Option<wgpu::TextureView>,
}

/// The attachments for a backing of `size` at `samples`.
pub(crate) fn attachments(
    device: &wgpu::Device,
    size: SurfaceSize,
    samples: Samples,
) -> Attachments {
    let colour = (samples == Samples::Four).then(|| {
        texture(
            device,
            (size, samples),
            ("scene3d multisampled", SURFACE_FORMAT),
        )
    });
    Attachments {
        depth: texture(device, (size, samples), ("scene3d depth", DEPTH_FORMAT)),
        colour,
    }
}

fn texture(
    device: &wgpu::Device,
    (size, samples): (SurfaceSize, Samples),
    (label, format): (&str, wgpu::TextureFormat),
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: samples.count(),
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

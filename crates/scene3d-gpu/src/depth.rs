//! The depth buffer, allocated at the backing's device size in `resize`.

use crate::pipeline::DEPTH_FORMAT;
use vapors_app::SurfaceSize;

/// A depth texture's view; the texture lives as long as it.
pub(crate) fn texture(device: &wgpu::Device, size: SurfaceSize) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("scene3d depth"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

//! A surface's backing texture: what its renderer draws into and the
//! painter composites.

use super::super::errors::Refusal;
use crate::surface::{SURFACE_FORMAT, SurfaceSize};

/// The texture of one active surface, and the size it was made at.
pub(super) struct Backing {
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) size: SurfaceSize,
}

impl Backing {
    pub(super) fn new(device: &wgpu::Device, size: SurfaceSize) -> Self {
        let texture = device.create_texture(&descriptor(size));
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            view,
            size,
        }
    }
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

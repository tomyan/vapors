//! The frame's depth attachment, made by the renderer so its format,
//! samples and size are known on the CPU (#493 review, M1): a pass
//! refuses one that is not the target's, naming both, before wgpu
//! would at submit without naming either.

use vapors_scene::Size;

use crate::{DEPTH_FORMAT, Samples};

/// A [`DEPTH_FORMAT`] texture of a size and sample count, and its view.
#[derive(Debug)]
pub struct DepthAttachment {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: Size,
    samples: Samples,
}

impl DepthAttachment {
    /// A depth attachment of `size` pixels at `samples`. At one sample it
    /// may also be copied from, so a test can read it back.
    #[must_use]
    pub fn new(device: &wgpu::Device, size: Size, samples: Samples) -> Self {
        let copy = match samples {
            Samples::One => wgpu::TextureUsages::COPY_SRC,
            Samples::Four => wgpu::TextureUsages::empty(),
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frame depth"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: samples.count(),
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | copy,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            view,
            size,
            samples,
        }
    }

    /// The view a pass attaches.
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// The texture, for reading back.
    #[must_use]
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// Its size in pixels.
    #[must_use]
    pub fn size(&self) -> Size {
        self.size
    }

    /// Its sample count.
    #[must_use]
    pub fn samples(&self) -> Samples {
        self.samples
    }
}

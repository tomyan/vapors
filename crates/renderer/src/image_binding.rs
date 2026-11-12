//! A texture bound for the image pipeline: its view, sampler and bind
//! group, made once and kept beside the texture by whoever holds it,
//! so a frame drawing the texture again makes nothing. The binding
//! owns the texture: dropping it drops the bind group with the texture,
//! so no bind group outlives the texture it was made for.

/// How the image pipeline samples a texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sampling {
    /// One image pixel per pixel at any zoom: the canvas.
    Nearest,
    /// Smoothly, with mips: an `img` stretched over its box.
    Smooth,
}

/// A texture and the image pipeline's bind group for it.
pub struct TextureBinding {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

impl TextureBinding {
    pub(crate) fn new(texture: wgpu::Texture, bind_group: wgpu::BindGroup) -> Self {
        Self {
            texture,
            bind_group,
        }
    }

    /// The bound texture.
    #[must_use]
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub(crate) fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

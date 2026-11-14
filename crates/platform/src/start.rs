//! What an app is given once, after its GPU is ready and before its
//! first frame (one-scene F0): the work a first frame would otherwise
//! do once, such as building pipelines, is done here, so no shown frame
//! pays for it.

/// The GPU and the surface the frames will draw into.
pub struct Start<'a> {
    /// The adapter drawing this window.
    pub adapter: &'a wgpu::Adapter,
    /// Device for this window.
    pub device: &'a wgpu::Device,
    /// Queue for this window.
    pub queue: &'a wgpu::Queue,
    /// The format of every frame's texture.
    pub format: wgpu::TextureFormat,
    /// The surface's width in device pixels.
    pub width: u32,
    /// The surface's height in device pixels.
    pub height: u32,
    /// Device pixels per logical pixel, as [`Frame::scale_factor`](crate::Frame::scale_factor).
    pub scale_factor: f64,
}

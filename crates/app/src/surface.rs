//! The renderer contract of a `gpu-surface` (docs/gpu-surface.md): what
//! an application implements to draw a surface's content on the GPU,
//! and what vapors hands it. The painter owns the backing texture, the
//! command encoder and submission order; the renderer owns what it
//! draws.

/// The backing texture's format: 8-bit RGBA holding sRGB-encoded values
/// with straight (not premultiplied) alpha, as an `img` texture does.
/// The painter composites it with ordinary alpha blending.
pub const SURFACE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// What a renderer reports when it cannot draw; the painter refuses the
/// frame with it, naming the surface.
pub type SurfaceError = Box<dyn std::error::Error>;

/// A surface's backing size: device pixels, and the scale they came from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceSize {
    /// Width in device pixels; at least 1.
    pub width: u32,
    /// Height in device pixels; at least 1.
    pub height: u32,
    /// Device pixels per CSS pixel.
    pub scale: f32,
}

impl SurfaceSize {
    /// The content box in CSS pixels: `[width, height]`.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // texture sizes are at most 2^14
    pub fn logical(&self) -> [f32; 2] {
        [
            self.width as f32 / self.scale,
            self.height as f32 / self.scale,
        ]
    }
}

/// One render of a surface.
pub struct SurfaceFrame<'a> {
    /// The application's device, shared with the painter.
    pub device: &'a wgpu::Device,
    /// The application's queue; the painter submits `encoder`.
    pub queue: &'a wgpu::Queue,
    /// Record the draws here; the painter submits it before it
    /// composites the surface. Submitting work of your own on `queue`
    /// is allowed, and runs before it.
    pub encoder: &'a mut wgpu::CommandEncoder,
    /// The backing texture, [`SURFACE_FORMAT`], `size` big. Its contents
    /// are kept between renders; a newly allocated one is transparent.
    pub view: &'a wgpu::TextureView,
    /// The backing size.
    pub size: SurfaceSize,
}

/// An application's renderer for one `gpu-surface`, mounted on the
/// painter by the surface's `name`.
///
/// Calls, in order: `mount` once, before anything else; `resize` when
/// the backing texture is allocated (on the first frame the surface is
/// painted, on a change of device size or scale, and when it is painted
/// again after a frame it was not); `render` after a `resize`, after
/// [`ScenePainter::invalidate`](crate::ScenePainter::invalidate), and on
/// every frame while `animating`. Unmounting drops the renderer: teardown
/// is `Drop`. A frame that does not need a render composites the last one.
///
/// A surface not painted in a frame is inactive: the painter releases its
/// backing, but the renderer is not told and keeps its own resources
/// (pipelines, buffers, a depth texture) until it is unmounted. A
/// `deactivate` hook called when the backing is released would let it
/// free them; there is none yet.
///
/// A GPU validation error made in `mount`, `resize` or `render` (the
/// encoder is submitted inside the same error scope) is refused as the
/// renderer's failure, naming the surface. Native wgpu reports it in the
/// frame that made it; the web reports it later, and the surface's next
/// frame is refused instead.
pub trait SurfaceRenderer {
    /// Creates persistent resources (pipelines, buffers) on the shared
    /// device.
    ///
    /// # Errors
    ///
    /// When they cannot be made; the frame is refused.
    fn mount(&mut self, device: &wgpu::Device) -> Result<(), SurfaceError> {
        let _ = device;
        Ok(())
    }

    /// Creates size-dependent resources, such as a depth buffer.
    ///
    /// # Errors
    ///
    /// When they cannot be made; the frame is refused.
    fn resize(&mut self, device: &wgpu::Device, size: SurfaceSize) -> Result<(), SurfaceError> {
        let _ = (device, size);
        Ok(())
    }

    /// Records the surface's content into `frame.encoder`.
    ///
    /// # Errors
    ///
    /// When it cannot draw; the frame is refused rather than shown with
    /// stale or blank content.
    fn render(&mut self, frame: SurfaceFrame<'_>) -> Result<(), SurfaceError>;

    /// True while the content changes every frame: the painter renders
    /// it each frame and asks for the next ([`ScenePainter::wants_frame`](crate::ScenePainter::wants_frame)).
    fn animating(&self) -> bool {
        false
    }
}

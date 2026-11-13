//! Drawing a run of the scene's quads: element backgrounds and solid
//! borders with circular corner radii, anti-aliased by the shader's
//! signed distance to the rounded edges.

use vapors_scene::{Color, Quad};

use crate::instanced::{Instanced, float_bytes};
use crate::{GpuWork, KeptRange, Target};

/// A pipeline that draws quads. Create once per target format.
pub struct QuadPipeline(Instanced);

impl QuadPipeline {
    /// Build the pipeline for targets of `format`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self(Instanced::new(
            device,
            format,
            "quads",
            include_str!("quads.wgsl"),
            8,
        ))
    }

    /// Draw `quads` into `target`, later quads over earlier ones, in one
    /// draw call. Nothing is recorded for an empty slice.
    pub fn draw(&mut self, target: &Target<'_>, quads: &[Quad]) {
        self.0.draw(target, &Self::bytes(quads));
    }

    /// The instance bytes of `quads`, as a draw writes them.
    #[must_use]
    pub fn bytes(quads: &[Quad]) -> Vec<u8> {
        float_bytes(quads.iter().flat_map(instance))
    }

    /// Writes `bytes` ([`QuadPipeline::bytes`] of a run) into the kept buffer,
    /// for [`QuadPipeline::draw_kept`] in this and later frames. `None`
    /// when the buffer they need is over the device's `max_buffer_size`:
    /// the refusal is on the target's encoder.
    pub fn keep(&mut self, target: &Target<'_>, bytes: &[u8]) -> Option<KeptRange> {
        self.0.keep(target, bytes)
    }

    /// Draws the quads `kept` holds into `target`, moved by `shift`
    /// (device pixels: a scroll frame's), in one draw call.
    ///
    /// # Panics
    ///
    /// If `kept` was kept by another pipeline.
    pub fn draw_kept(&mut self, target: &Target<'_>, kept: &KeptRange, shift: [f32; 2]) {
        self.0.draw_kept(target, kept, shift);
    }

    /// Gives `kept`'s bytes back. Draws of it already recorded still
    /// read them: they are reused once a newer encoder writes or draws.
    ///
    /// # Panics
    ///
    /// If `kept` was kept by another pipeline.
    pub fn release(&mut self, kept: KeptRange) {
        self.0.release(kept);
    }

    /// Bytes held in the kept buffer.
    #[must_use]
    pub fn retained_bytes(&self) -> u64 {
        self.0.retained_bytes()
    }

    /// The bind groups kept: one once this pipeline has drawn.
    #[must_use]
    pub fn bind_groups_kept(&self) -> u64 {
        self.0.bind_groups_kept()
    }

    /// The work its draws issued since the last take; the count restarts.
    pub fn take_work(&self) -> GpuWork {
        self.0.work.take()
    }
}

/// The quad's eight four-float attributes, as `quads.wgsl` reads them.
fn instance(quad: &Quad) -> Vec<f32> {
    let r = quad.rect;
    let mut floats = vec![r.left, r.top, r.right, r.bottom];
    floats.extend(rgba(quad.background));
    floats.extend(quad.border);
    for color in quad.border_colors {
        floats.extend(rgba(color));
    }
    floats.extend(quad.radii);
    floats
}

pub(crate) fn rgba(color: Color) -> [f32; 4] {
    [color.r, color.g, color.b, color.a]
}

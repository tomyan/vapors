//! Drawing a run of closed-form shapes (docs/plans/one-scene.md 1.2): a
//! rect or ellipse in its own units under an affine transform, filled or
//! stroked, with coverage from its signed distance in device pixels.
//! The ramp straddles the true outline, 0.5 on it (Tom, 2026-11-13, as
//! Chromium draws an edge), so each instance's quad is grown by a
//! device pixel past the shape for the outer half's fragments.

use crate::instanced::{Instanced, float_bytes};
use crate::{ClosedFormShape, GpuWork, KeptRange, Shift, Target};

/// A pipeline that draws closed-form shapes. Create once per target
/// format.
pub struct ClosedFormPipeline(Instanced);

impl ClosedFormPipeline {
    /// Build the pipeline for targets of `format`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self(Instanced::new(
            device,
            format,
            "closed forms",
            include_str!("closed_forms.wgsl"),
            5,
        ))
    }

    /// Draw `shapes` into `target`, later ones over earlier ones, in one
    /// draw call. Nothing is recorded for an empty slice.
    pub fn draw(&mut self, target: &Target<'_>, shapes: &[ClosedFormShape]) {
        self.0.draw(target, &Self::bytes(shapes));
    }

    /// The instance bytes of `shapes`, as a draw writes them.
    #[must_use]
    pub fn bytes(shapes: &[ClosedFormShape]) -> Vec<u8> {
        float_bytes(shapes.iter().flat_map(ClosedFormShape::instance))
    }

    /// Writes `bytes` ([`ClosedFormPipeline::bytes`] of a run) into the
    /// kept buffer, for [`ClosedFormPipeline::draw_kept`] in this and
    /// later frames. `None` when the buffer they need is over the
    /// device's `max_buffer_size`: the refusal is on the target's
    /// encoder.
    pub fn keep(&mut self, target: &Target<'_>, bytes: &[u8]) -> Option<KeptRange> {
        self.0.keep(target, bytes)
    }

    /// Draws the shapes `kept` holds into `target`, moved by `shift`
    /// (device pixels: a scroll frame's), in one draw call.
    ///
    /// # Panics
    ///
    /// If `kept` was kept by another pipeline.
    pub fn draw_kept(&mut self, target: &Target<'_>, kept: &KeptRange, shift: Shift) {
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

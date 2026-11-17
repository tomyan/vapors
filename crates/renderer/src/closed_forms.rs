//! Drawing a run of closed-form shapes (docs/plans/one-scene.md 1.2): a
//! rect or ellipse in its own units under an affine transform, filled or
//! stroked, with coverage from its signed distance in device pixels.
//! The ramp straddles the true outline, 0.5 on it (Tom, 2026-11-13, as
//! Chromium draws an edge), so each instance's quad is grown by a
//! device pixel past the shape for the outer half's fragments.

use crate::instanced::{Instanced, instance_bytes, instanced_source, rowed_instances};
use crate::{ClipTable, ClosedFormShape, GpuWork, KeptRange, Target, TransformTable};

/// A pipeline that draws closed-form shapes. Create once per target
/// format.
pub struct ClosedFormPipeline(Instanced);

impl ClosedFormPipeline {
    /// Four-float attributes an instance holds.
    const VEC4S: u32 = 5;

    /// Bytes per instance in a draw's or a kept buffer: the attributes,
    /// then the row's index and the frame's entry (`instanced.rs`).
    pub const INSTANCE_BYTES: usize = instance_bytes(Self::VEC4S);

    /// Build the pipeline for passes of `pass` (a format alone is one
    /// sample).
    #[must_use]
    pub fn new(device: &wgpu::Device, pass: impl Into<crate::PassFormat>) -> Self {
        Self(Instanced::new(
            device,
            pass.into(),
            "closed forms",
            &instanced_source(concat!(
                include_str!("closed_forms_2d.wgsl"),
                include_str!("closed_forms_shared.wgsl")
            )),
            Self::VEC4S,
        ))
    }

    /// Draw `shapes` into `target`, later ones over earlier ones, in one
    /// draw call. Nothing is recorded for an empty slice.
    pub fn draw(&mut self, target: &Target<'_>, shapes: &[ClosedFormShape]) {
        self.0.draw(target, &Self::bytes(shapes));
    }

    /// The instance bytes of `shapes`, as a draw writes them: each on
    /// the page.
    #[must_use]
    pub fn bytes(shapes: &[ClosedFormShape]) -> Vec<u8> {
        Self::bytes_in(shapes, (0, &[]), &[])
    }

    /// The instance bytes of `shapes`, each in frame `frame` (its entry
    /// in the transform table), item `i` in row `rows[i]` of its batch's
    /// range, or every item in row 0 for an empty `rows`.
    ///
    /// # Panics
    ///
    /// As [`QuadPipeline::bytes_in`](crate::QuadPipeline::bytes_in).
    #[must_use]
    pub fn bytes_in(
        shapes: &[ClosedFormShape],
        (frame, clips): (u32, crate::ClipRuns<'_>),
        rows: &[u32],
    ) -> Vec<u8> {
        rowed_instances(
            ("closed forms", Self::VEC4S as usize),
            (shapes, (frame, clips), rows),
            |shape| shape.instance().to_vec(),
        )
    }

    /// Writes `bytes` ([`ClosedFormPipeline::bytes`] of a run) into the
    /// kept buffer, for [`ClosedFormPipeline::draw_kept`] in this and
    /// later frames. `None` when the buffer they need is over the
    /// device's `max_buffer_size`: the refusal is on the target's
    /// encoder.
    pub fn keep(&mut self, target: &Target<'_>, bytes: &[u8]) -> Option<KeptRange> {
        self.0.keep(target, bytes)
    }

    /// Draws the shapes `kept` holds into `target`, each at its frame's
    /// entry in `table` and its row's in its batch's range from
    /// `rows_base`, in one draw call.
    ///
    /// # Panics
    ///
    /// If `kept` was kept by another pipeline.
    pub fn draw_kept(
        &mut self,
        target: &Target<'_>,
        kept: &KeptRange,
        table: (&mut TransformTable, &mut ClipTable, u32),
    ) {
        self.0.draw_kept(target, kept, table);
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

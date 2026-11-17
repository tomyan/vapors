//! An image pipeline's quads kept between frames: a run's quads written
//! once into its kept buffer and drawn from it, one draw per quad with
//! its own texture, until released (`retained.rs`).

use std::ops::Range;

use vapors_scene::Rect;

use super::{ImagePipeline, Quads};
use crate::image_binding::TextureBinding;
use crate::quad::{VERTEX_COUNT, VERTEX_SIZE, quad_corners, vertex_bytes};
use crate::{ClipTable, KeptRange, Target, TransformTable};

/// Where a kept quad is drawn: the transform table and its frame's and
/// row's entries in it, and the clip table and its clip's entry.
pub type ImagePlace<'a> = ((&'a mut TransformTable, u32, u32), (&'a mut ClipTable, u32));

impl ImagePipeline {
    /// The vertex bytes of a quad over each of `rects` (device pixels),
    /// in order, as [`ImagePipeline::draw_rect`] writes one.
    #[must_use]
    pub fn rect_bytes(rects: impl IntoIterator<Item = Rect>) -> Vec<u8> {
        let quad = |rect: Rect| [[rect.left, rect.top], [rect.right, rect.bottom]];
        rects
            .into_iter()
            .flat_map(|rect| vertex_bytes(&quad_corners(quad(rect))))
            .collect()
    }

    /// Writes `bytes` ([`ImagePipeline::rect_bytes`]) into the kept
    /// buffer, for [`ImagePipeline::draw_kept`] in this and later
    /// frames, on targets of any viewport. `None` when the buffer they
    /// need is over the device's `max_buffer_size`: the refusal is on
    /// the target's encoder.
    pub fn keep(&mut self, target: &Target<'_>, bytes: &[u8]) -> Option<KeptRange> {
        let (frame, queue) = (target.encoder, target.queue);
        self.kept.keep(frame, queue, &self.work, bytes)
    }

    /// Draws `bound`'s texture over the `index`th quad `kept` holds, at
    /// its frame's entry `frame` in `table` and its row's entry `row` (0,
    /// the identity, for a quad in no lane row), under its clip's entry
    /// `clip` in `clips` (0 for none), in `target`; not when the view slot
    /// is refused (the refusal is on the encoder).
    ///
    /// # Panics
    ///
    /// If `kept` was kept by another pipeline or holds no `index`th quad,
    /// or `table` or `clips` is not the table this pipeline's kept draws
    /// bind (one of each a pipeline).
    pub fn draw_kept(
        &mut self,
        target: &Target<'_>,
        bound: &TextureBinding,
        (kept, index): (&KeptRange, u32),
        ((table, frame, row), (clips, clip)): ImagePlace<'_>,
    ) {
        let vertices = self.vertices(kept, index);
        let Some(view) = self.view_slot(target, (frame, row, clip)) else {
            return;
        };
        self.adopt_clips(clips);
        let id = *self.table_id.get_or_insert(table.id());
        assert_eq!(
            id,
            table.id(),
            "image quads: a kept draw binds transform table {}, after table {id}: a pipeline draws \
             from one table (image_kept.rs); give each painter its own pipelines; reproduce \
             with `cargo xtask test`",
            table.id()
        );
        table.prepare(target.queue);
        clips.prepare(target.queue);
        self.record(
            target,
            bound,
            (Quads::Kept(kept), vertices),
            (Some((table, clips)), view),
        );
    }

    /// Panics unless `clips` is the clip table this pipeline's kept draws
    /// bind: one a pipeline, as with the transform table.
    fn adopt_clips(&mut self, clips: &ClipTable) {
        let id = *self.clips_id.get_or_insert(clips.id());
        assert_eq!(
            id,
            clips.id(),
            "image quads: a kept draw binds clip table {}, after table {id}: a pipeline draws \
             from one clip table (image_kept.rs); give each painter its own pipelines; \
             reproduce with `cargo xtask test`",
            clips.id()
        );
    }

    /// The vertices of the `index`th quad `kept` holds.
    ///
    /// # Panics
    ///
    /// As [`ImagePipeline::draw_kept`].
    fn vertices(&self, kept: &KeptRange, index: u32) -> Range<u32> {
        let quads = self.kept.len(kept) / VERTEX_SIZE / u64::from(VERTEX_COUNT);
        assert!(
            u64::from(index) < quads,
            "image quads: quad {index} drawn from a kept range of {quads} quads; draw each \
             kept range's quads by their place in it (image_kept.rs)"
        );
        let first = index * VERTEX_COUNT;
        first..first + VERTEX_COUNT
    }

    /// Gives `kept`'s bytes back. Draws of it already recorded still
    /// read them: they are reused once a newer encoder writes or draws.
    ///
    /// # Panics
    ///
    /// If `kept` was kept by another pipeline.
    pub fn release(&mut self, kept: KeptRange) {
        self.kept.release(kept);
    }

    /// Bytes held in the kept buffer.
    #[must_use]
    pub fn retained_bytes(&self) -> u64 {
        self.kept.live_bytes()
    }
}

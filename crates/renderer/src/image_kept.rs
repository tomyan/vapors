//! An image pipeline's quads kept between frames: a run's quads written
//! once into its kept buffer and drawn from it, one draw per quad with
//! its own texture, until released (`retained.rs`).

use vapors_scene::{Rect, Size};

use super::ImagePipeline;
use crate::image_binding::TextureBinding;
use crate::quad::{VERTEX_COUNT, VERTEX_SIZE, quad_vertices, vertex_bytes};
use crate::{KeptRange, Target};

impl ImagePipeline {
    /// The vertex bytes of a quad over each of `rects` (device pixels)
    /// on a target of `viewport`, in order, as [`ImagePipeline::draw_rect`]
    /// writes one.
    #[must_use]
    pub fn rect_bytes(viewport: Size, rects: impl IntoIterator<Item = Rect>) -> Vec<u8> {
        let quad = |rect: Rect| [[rect.left, rect.top], [rect.right, rect.bottom]];
        rects
            .into_iter()
            .flat_map(|rect| vertex_bytes(&quad_vertices(quad(rect), viewport)))
            .collect()
    }

    /// Writes `bytes` ([`ImagePipeline::rect_bytes`]) into the kept
    /// buffer, for [`ImagePipeline::draw_kept`] in this and later
    /// frames on targets of the viewport they were made for. `None` when
    /// the buffer they need is over the device's `max_buffer_size`: the
    /// refusal is on the target's encoder.
    pub fn keep(&mut self, target: &Target<'_>, bytes: &[u8]) -> Option<KeptRange> {
        let (frame, queue) = (target.encoder, target.queue);
        self.kept.keep(frame, queue, &self.work, bytes)
    }

    /// Draws `bound`'s texture over the `index`th quad `kept` holds, in
    /// `target`, inside the target's scissor.
    ///
    /// # Panics
    ///
    /// If `kept` was kept by another pipeline.
    pub fn draw_kept(
        &mut self,
        target: &Target<'_>,
        bound: &TextureBinding,
        (kept, index): (&KeptRange, u32),
    ) {
        let _encoding = tracing::trace_span!("draw step", phase = "encode").entered();
        let first = index * VERTEX_COUNT;
        let held = self.kept.len(kept) / VERTEX_SIZE;
        assert!(
            u64::from(first + VERTEX_COUNT) <= held,
            "image quads: quad {index} drawn from a kept range of {} quads; draw each kept \
             range's quads by their place in it (image_kept.rs)",
            held / u64::from(VERTEX_COUNT)
        );
        let vertices = self.kept.slice(kept);
        target.draw_pass(|pass| {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bound.bind_group(), &[]);
            pass.set_vertex_buffer(0, vertices);
            self.work.draw(pass, first..first + VERTEX_COUNT, 0..1);
        });
    }

    /// Gives `kept`'s bytes back. Release a range only in a frame that
    /// does not draw it.
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

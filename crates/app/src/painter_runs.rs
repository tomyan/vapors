//! Drawing the scene's kept batches (`painter.rs`): each batch in
//! painting order, moved by its scroll frame's shift, under its clip
//! moved the same way and cut to the frame's clip.

use vapors_elements::{BatchKey, KeptBatched, Painted};
use vapors_renderer::{Shift, Target};
use vapors_scene::Run;

use super::ScenePainter;
use super::errors::PaintError;
use super::kept::Kind as KeptKind;
use super::region::{clipped, on_page, view};

impl ScenePainter {
    pub(super) fn paint_runs(
        &mut self,
        target: &Target<'_>,
        painted: &Painted,
    ) -> Result<(), PaintError> {
        let batching = tracing::trace_span!("draw step", phase = "batches").entered();
        let kept = painted.kept_batches(view(target));
        drop(batching);
        self.counts.culled = kept.batched.culled;
        assert_parallel(&kept);
        let batches = kept.batched.batches.iter().zip(&kept.frames);
        for (at, ((batch, &frame), &key)) in batches.zip(&kept.keys).enumerate() {
            let frame = painted.frame_draw(frame);
            let clip = on_page(batch.clip, frame);
            match clipped(target, clip).filter(|_| !batch.culled) {
                Some(clipped) => {
                    self.draw_run(
                        &clipped,
                        painted,
                        (&batch.run, key, at),
                        Shift::whole(frame.shift),
                    )?;
                }
                None => self
                    .surfaces
                    .check_clipped(target.device, painted, &batch.run)?,
            }
        }
        Ok(())
    }

    /// Draws `run` with the pipeline for its kind under `target`'s
    /// scissor, moved by its frame's `shift`, and counts the work it
    /// issued. A draw the pipeline's buffers refused ends the frame here.
    fn draw_run(
        &mut self,
        target: &Target<'_>,
        painted: &Painted,
        (run, key, at): (&Run, BatchKey, usize),
        shift: Shift,
    ) -> Result<(), PaintError> {
        let result = match KeptKind::of_run(run) {
            Some(_) => self.draw_retained(target, painted, (run, key, at), shift),
            None => self.draw_kept_text(target, painted, (run, key, at), shift),
        };
        self.count_run(run);
        result?;
        target
            .encoder
            .take_refusal()
            .map_or(Ok(()), |refused| Err(refused.into()))
    }
}

/// Panics unless `kept` has a frame and a key for each batch: the
/// painter reads them side by side, and a shorter one would end the
/// draw early with no error.
fn assert_parallel(kept: &KeptBatched) {
    let lengths = (
        kept.batched.batches.len(),
        kept.frames.len(),
        kept.keys.len(),
    );
    assert!(
        lengths.0 == lengths.1 && lengths.0 == lengths.2,
        "the kept batches have {} batches, {} frames and {} keys: KeptBatched::keyed must give \
         each batch its frame and key (vapors_elements kept_batches_keys.rs); reproduce with \
         `cargo xtask test`",
        lengths.0,
        lengths.1,
        lengths.2
    );
}

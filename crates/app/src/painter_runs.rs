//! Drawing the scene's kept batches (`painter.rs`): each batch in
//! painting order, moved by its scroll frame's shift, under its clip
//! moved the same way and cut to the frame's clip; a batch over a
//! lane's rows moves each item by its row's offset too
//! (`painter_rows.rs`).

use vapors_elements::{BatchKey, KeptBatched, Painted};
use vapors_renderer::{Shift, Target};
use vapors_scene::Run;

use super::ScenePainter;
use super::errors::PaintError;
use super::kept::Kind as KeptKind;
use super::places::BatchPlaces;
use super::region::{clipped, view};
use super::rows::Moved;

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
        self.placed(painted, view(target), &kept);
        let mut places = std::mem::take(&mut self.places);
        let drawn = self.draw_batches(target, painted, (&kept, &mut places));
        self.places = places;
        drawn
    }

    /// Draws each of `kept`'s batches at its place in `places`, under its
    /// scissor; a clipped-out batch's surfaces checked instead.
    fn draw_batches(
        &mut self,
        target: &Target<'_>,
        painted: &Painted,
        (kept, places): (&KeptBatched, &mut BatchPlaces),
    ) -> Result<(), PaintError> {
        for (at, batch) in kept.batched.batches.iter().enumerate() {
            let frame = places.frame(at, painted, kept);
            let Some(clipped) =
                clipped(target, frame.on_page(batch.clip)).filter(|_| !batch.culled)
            else {
                self.surfaces
                    .check_clipped(target.device, painted, &batch.run)?;
                continue;
            };
            let scissor = Self::scissor((target, &clipped), batch, frame);
            self.draw_placed(&scissor, (painted, kept), (places, at, frame))?;
        }
        Ok(())
    }

    /// The scissor batch `batch` is drawn under: for quads,
    /// shadows and shapes, only the frame's clip, their own clip coming
    /// from the clip table as coverage (flat-scene F3.1); for the rest,
    /// `batch_scissor`, the batch's clip on the page, as before, until F3.1b.
    fn scissor<'t>(
        (target, batch_scissor): (&Target<'t>, &Target<'t>),
        batch: &vapors_scene::Batch,
        frame: vapors_elements::FrameDraw,
    ) -> Target<'t> {
        if on_the_clip_table(&batch.run) {
            return clipped(target, frame.on_page(None))
                .expect("inside the batch's clip on the page, so inside the frame's clip");
        }
        *batch_scissor
    }

    /// Draws `kept`'s batch `at` under `target` at `frame`, each row at
    /// its offset in `places`.
    fn draw_placed(
        &mut self,
        target: &Target<'_>,
        (painted, kept): (&Painted, &KeptBatched),
        (places, at, frame): (&mut BatchPlaces, usize, vapors_elements::FrameDraw),
    ) -> Result<(), PaintError> {
        let (offsets, table) = places.rows(at, (painted, kept), &mut self.row_tables);
        let moved = Moved {
            frame: kept.frames[at],
            shift: Shift::whole(frame.shift),
            table,
            offsets,
            rows: kept.rows.get(at).unwrap_or_default(),
            clip: kept.batched.batches[at].clip,
            draw: frame,
        };
        let run = &kept.batched.batches[at].run;
        self.draw_run(target, painted, (run, kept.keys[at], at), &moved)
    }

    /// Each batch's place for this frame: kept from the last when the
    /// paint and view are the same, else worked out again as each batch
    /// is drawn, each lane batch's table kept by key (`painter_rows.rs`).
    fn placed(&mut self, painted: &Painted, view: vapors_scene::Rect, kept: &KeptBatched) {
        if self.places.current(painted, view, kept) {
            #[cfg(debug_assertions)]
            self.places.assert_current(painted, kept);
            return;
        }
        self.row_tables.begin();
        self.places.reset((painted, view), kept);
    }

    /// Draws `run` with the pipeline for its kind under `target`'s
    /// scissor, moved as `moved` says, and counts the work it issued. A
    /// draw the pipeline's buffers refused ends the frame here.
    fn draw_run(
        &mut self,
        target: &Target<'_>,
        painted: &Painted,
        (run, key, at): (&Run, BatchKey, usize),
        moved: &Moved<'_>,
    ) -> Result<(), PaintError> {
        let result = match KeptKind::of_run(run) {
            Some(_) => self.draw_retained(target, painted, (run, key, at), moved),
            None => self.draw_kept_text(target, painted, (run, key, at), moved),
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

/// Whether `run`'s items take their clip from the clip table (flat-scene
/// F3.1): quads, shadows and closed-form shapes, the pipelines on F2.1's
/// tables.
fn on_the_clip_table(run: &Run) -> bool {
    matches!(run, Run::Quads(_) | Run::Shadows(_) | Run::Shapes(_))
}

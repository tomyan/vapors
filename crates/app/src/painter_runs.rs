//! Drawing the scene's kept batches (`painter.rs`): each batch in
//! painting order, moved by its scroll frame's shift, under its clip
//! moved the same way and cut to the frame's clip; a batch over a
//! lane's rows moves each item by its row's offset too
//! (`painter_rows.rs`).

use vapors_elements::{BatchKey, KeptBatched, Painted};
use vapors_renderer::Target;
use vapors_scene::{Clip, Clips, Rect, Run};

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
                clipped(target, frame.on_page(batch.clips.union())).filter(|_| !batch.culled)
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

    /// The scissor batch `batch` is drawn under: for the runs on the
    /// clip table (quads, shadows, shapes, text and icons), only the
    /// frame's clip, their own clips coming from the table as coverage
    /// (flat-scene F3.1, F3.1b); for images, surfaces and meshes,
    /// `batch_scissor`, the batch's clip on the page, until the rest of
    /// F3.1b moves them onto the table.
    fn scissor<'t>(
        (target, batch_scissor): (&Target<'t>, &Target<'t>),
        batch: &vapors_scene::Batch,
        frame: vapors_elements::FrameDraw,
    ) -> Target<'t> {
        if scissor_clip(&batch.run, &batch.clips).is_none() {
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
        let (offsets, rows_key) = places.rows(at, (painted, kept));
        let moved = Moved {
            frame: kept.frames[at],
            shift: frame.shift,
            offsets,
            rows_key,
            rows: kept.rows.get(at).unwrap_or_default(),
            clips: &kept.batched.batches[at].clips,
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

/// The clip of a batch's own, of its items' `clips`, that its scissor
/// cuts to: none for a run on the clip table ([`Run::clipped_by_table`],
/// the one answer), whose scissor is its frame's clip only; the one clip
/// every item has for the rest.
///
/// # Panics
///
/// When a run the scissor clips has items under more than one clip: the
/// scissor would widen to their union and let each item past its own
/// (vapors-scene `batch.rs` `push` joins only runs on the clip table).
pub(super) fn scissor_clip(run: &Run, clips: &Clips) -> Option<Rect> {
    if run.clipped_by_table() {
        return None;
    }
    let clip = clips.uniform().unwrap_or_else(|| {
        panic!(
            "a {run:?} batch the scissor clips has items under several clips {:?}: only runs on \
             the clip table join across clips (vapors-scene batch.rs `push`); reproduce with \
             `cargo xtask test`",
            clips.runs()
        )
    })?;
    assert!(
        !clip.is_rounded(),
        "a {run:?} batch the scissor clips is under the rounded clip {clip:?}: a scissor cuts \
         square, so only runs on the clip table can be rounded (vapors-elements is to refuse \
         an image, surface or mesh under one from F3.2b, until F3.1b moves them onto the \
         table); reproduce with `cargo xtask test`"
    );
    Some(clip.rect())
}

/// The clip a batch's items under `clip` read from the clip table: `clip`
/// for a run on the table; none, entry 0, for the rest
/// (`painter_kept_entries.rs`).
pub(super) fn table_clip(run: &Run, clip: Option<Clip>) -> Option<Clip> {
    clip.filter(|_| run.clipped_by_table())
}

#[cfg(test)]
#[path = "painter_runs_tests.rs"]
mod tests;

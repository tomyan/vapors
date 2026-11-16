//! Where each batch of a frame is drawn, kept for the next frame
//! (`painter_runs.rs`; `docs/plans/splice-layout.md` section 11.2,
//! 4f-2d): each batch's frame draw (shift and clip), its rows' offsets
//! and their table, made again only when the paint, the view or the
//! batches changed. An idle frame reads them, and works out no frame, no
//! offset and no table.

use vapors_elements::{FrameDraw, KeptBatched, Painted};
use vapors_renderer::RowOffsets;
use vapors_scene::Rect;

use super::rows::RowTables;

/// Each batch's place, worked out when the batch is first drawn after a
/// new paint or view, and what they were made for.
#[derive(Default)]
pub(super) struct BatchPlaces {
    made_for: Option<(u64, Rect)>,
    places: Vec<Place>,
    offsets: Vec<[f32; 2]>,
    /// The lane batches' tables, each a kilobyte: kept here once, read
    /// by reference, so a place stays a few words.
    tables: Vec<RowOffsets>,
    /// The keys of the batches the places were reset for, for the debug
    /// check that the batches drawn are those.
    #[cfg(debug_assertions)]
    keys: Vec<vapors_elements::BatchKey>,
    /// By lane part: its rows' offsets and table, once worked out, which
    /// each batch of the part shares (`RowsOfBatches::part`).
    parts: Vec<Option<Rows>>,
}

/// A batch's rows' offsets, a range of `BatchPlaces::offsets`, and their
/// table in `BatchPlaces::tables` (none for a batch in one frame).
type Rows = ((usize, usize), Option<usize>);

/// Where one batch is drawn, each part once worked out: its frame's (or
/// lane template's) shift and clip; its rows' offsets in
/// `BatchPlaces::offsets` and their table in `BatchPlaces::tables` (none
/// for a batch in one frame; only for a batch drawn).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Place {
    frame: Option<FrameDraw>,
    rows: Option<Rows>,
}

impl BatchPlaces {
    /// Whether the places are those of the batches drawn at `view` from
    /// `painted`: the same paint's frames and view. With neither changed
    /// the kept batches are the same ones (`KeptBatches::for_view`); the
    /// debug check `assert_current` holds them to the paint each frame.
    pub(super) fn current(&self, painted: &Painted, view: Rect, kept: &KeptBatched) -> bool {
        self.made_for == Some((painted.frames_epoch(), view))
            && self.places.len() == kept.frames.len()
    }

    /// Places for `count` batches drawn at `view` from `painted`, none
    /// worked out yet.
    pub(super) fn reset(&mut self, (painted, view): (&Painted, Rect), kept: &KeptBatched) {
        self.made_for = Some((painted.frames_epoch(), view));
        #[cfg(debug_assertions)]
        self.keys.clone_from(&kept.keys);
        self.places.clear();
        self.places.resize(kept.frames.len(), Place::default());
        self.offsets.clear();
        self.tables.clear();
        self.parts.clear();
    }

    /// Where `kept`'s batch `at` is drawn, from `painted` when not yet
    /// worked out.
    pub(super) fn frame(&mut self, at: usize, painted: &Painted, kept: &KeptBatched) -> FrameDraw {
        let place = self.slot(at);
        *place
            .frame
            .get_or_insert_with(|| painted.frame_draw(kept.frames[at]))
    }

    /// The offsets of `kept`'s batch `at`'s rows and their table, from
    /// `painted` and `tables` when not yet worked out.
    pub(super) fn rows(
        &mut self,
        at: usize,
        (painted, kept): (&Painted, &KeptBatched),
        tables: &mut RowTables,
    ) -> (&[[f32; 2]], &RowOffsets) {
        if self.slot(at).rows.is_none() {
            let part = kept.rows.part(at);
            let shared = part.and_then(|part| self.parts.get(part).copied().flatten());
            let rows = shared.unwrap_or_else(|| self.work_out_rows(at, (painted, kept), tables));
            if let Some(part) = part {
                if self.parts.len() <= part {
                    self.parts.resize(part + 1, None);
                }
                self.parts[part] = Some(rows);
            }
            self.slot(at).rows = Some(rows);
        }
        let ((start, end), table) = self.slot(at).rows.expect("worked out above");
        let table = table.map_or(&RowOffsets::NONE, |table| &self.tables[table]);
        (&self.offsets[start..end], table)
    }

    /// `kept`'s batch `at`'s rows' offsets and table worked out and kept.
    fn work_out_rows(
        &mut self,
        at: usize,
        (painted, kept): (&Painted, &KeptBatched),
        tables: &mut RowTables,
    ) -> Rows {
        let rows = kept.rows.get(at).unwrap_or_default();
        let offsets = painted.row_offsets(kept.frames[at], rows);
        let table = (!offsets.is_empty()).then(|| {
            self.tables.push(tables.table(kept.keys[at], &offsets));
            self.tables.len() - 1
        });
        let start = self.offsets.len();
        self.offsets.extend(offsets);
        ((start, self.offsets.len()), table)
    }

    /// Batch `at`'s place.
    ///
    /// # Panics
    ///
    /// When `at` is past the batches the places were reset for.
    fn slot(&mut self, at: usize) -> &mut Place {
        let count = self.places.len();
        self.places.get_mut(at).unwrap_or_else(|| {
            panic!(
                "batch {at} has no place: places for {count} were reset (painter_places.rs \
                 `reset` runs for each new paint or view); reproduce with `cargo xtask test`"
            )
        })
    }

    /// Panics, naming the batch, when a kept place is not where `painted`
    /// draws `kept`'s batch now: the frames changed under the same epoch.
    #[cfg(debug_assertions)]
    pub(super) fn assert_current(&self, painted: &Painted, kept: &KeptBatched) {
        assert!(
            self.keys == kept.keys,
            "the places kept were made for batches {:?}, and batches {:?} are drawn under the same \
             paint and view: the batches were made again with no new paint (painter_places.rs \
             `current`); reproduce with `cargo xtask test`",
            self.keys,
            kept.keys
        );
        for (at, place) in self.places.iter().enumerate() {
            let named = kept.frames[at];
            let held = place
                .rows
                .map(|((start, end), _)| &self.offsets[start..end]);
            let fresh = || painted.row_offsets(named, kept.rows.get(at).unwrap_or_default());
            let frame_held = place
                .frame
                .is_none_or(|frame| frame == painted.frame_draw(named));
            assert!(
                frame_held && held.is_none_or(|held| held == fresh()),
                "batch {at}'s kept place {place:?} is not where the paint draws it now \
                 ({:?}, {:?}): the frames changed under the same epoch (frames_lane.rs \
                 `next_epoch`, set by every paint); reproduce with `cargo xtask test`",
                painted.frame_draw(named),
                fresh()
            );
        }
    }
}

#[cfg(test)]
#[path = "painter_places_tests.rs"]
mod tests;

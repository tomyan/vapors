//! Batches kept across draws (IU3 3c-3f, `docs/plans/iu3.md` "3c-3f:
//! kept batches"): for the view of the last draw, the drawing steps
//! that show something in it, the culled count of the others, and the
//! batches of the shown steps' items. A draw with the same view reads
//! the kept batches; an update patches the steps of the slots it paints
//! again (`paint_partial.rs`), and the batches made again after it keep
//! the keys of those whose runs did not change (`kept_batches_keys.rs`).
//! Items stay in their scroll frames' coordinates (`framed_batches.rs`).
//! When a frame's shift or clip changes, or paint order is walked again,
//! the steps are sorted again at the next draw and the batches made
//! again, keeping the keys of equal runs. A debug build compares every
//! draw's batches with batching the whole scene afresh.

use vapors_scene::Rect;

use crate::drawn_steps::DrawnSteps;
use rustc_hash::FxHashMap;

use crate::frame_extents::Extents;
use crate::framed_batches::{RowSlots, framed_batches};
use crate::frames::Frames;
use crate::kept_batches_keys::KeptBatched;
use crate::kept_box::KeptBox;
use crate::paint_labels::Label;
use crate::paint_order::Step;

#[cfg(test)]
thread_local! {
    /// How many times [`KeptBatches::of`] ran on this thread, for tests
    /// that a draw reuses the kept batches.
    pub(crate) static BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// What kept batches are sorted and made from: the steps that draw,
/// their extents index, each slot's kept paint and the scroll frames.
#[derive(Clone, Copy)]
pub(crate) struct Sources<'a> {
    /// Read only by the debug build's check of the steps found
    /// (`kept_batches_check.rs` `assert_candidates`).
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    pub(crate) drawn: &'a DrawnSteps,
    pub(crate) extents: &'a Extents,
    pub(crate) boxes: &'a [Option<KeptBox>],
    pub(crate) frames: &'a Frames,
}

/// A shown step: its label, the step, how many items it paints, and its
/// frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Shown {
    pub(crate) label: Label,
    pub(crate) step: Step,
    pub(crate) items: u64,
    pub(crate) frame: usize,
}

/// The shown steps and batches for one view.
#[derive(Debug)]
pub(crate) struct KeptBatches {
    pub(crate) view: Rect,
    /// The drawing steps with an item in `view` or a surface, ordered by
    /// label.
    shown: Vec<Shown>,
    /// The items of `shown`'s steps.
    shown_items: u64,
    /// Of those, the items culled when `batched` was made.
    shown_culled: u64,
    /// The batches of `shown`'s items, with every other drawing step's
    /// items counted culled, and their keys; `None` until the first draw.
    batched: Option<KeptBatched>,
    /// True when a shown step changed since `batched` was made: the
    /// next draw makes them again, keeping the keys of equal runs.
    stale: bool,
    /// True when `shown` no longer holds: a frame's shift or clip
    /// changed, or paint order was walked. The next draw sorts the
    /// steps again, keeping `batched` for its keys.
    unsorted: bool,
    /// Each framed lane in the view, by container, as the last search
    /// met it: what a search after lane rows moved compares its ends with
    /// (`kept_batches_search.rs`).
    lanes: FxHashMap<usize, crate::frames::LaneSeen>,
    /// Each framed lane's rows' slots in its parts' tables, kept while
    /// the rows are shown (`framed_batches_slots.rs`).
    slots: RowSlots,
}

impl KeptBatches {
    /// `slot`'s kept batches for `view`, sorted from `sources` when it
    /// holds none, holds them for another view or unsorted; with their
    /// batches made, keeping the keys of equal runs of the last ones.
    pub(crate) fn for_view(slot: &mut Option<Self>, view: Rect, sources: Sources<'_>) {
        if slot
            .as_ref()
            .is_some_and(|kept| kept.view != view || kept.unsorted)
        {
            let (batched, slots) = slot
                .take()
                .map_or_else(Default::default, |kept| (kept.batched, kept.slots));
            *slot = Some(Self {
                batched,
                slots,
                ..Self::of(view, sources)
            });
        }
        let kept = slot.get_or_insert_with(|| Self::of(view, sources));
        // Every draw, sorted afresh or patched (a splice, lane rows
        // moved, slots painted again): the steps a full search finds.
        #[cfg(debug_assertions)]
        crate::kept_batches_check::assert_candidates(&kept.shown, view, sources);
        kept.batches(sources);
    }

    /// Whether the steps are to be sorted again at the next draw
    /// (`UpdateCounts::batch_steps_sorted_again`).
    pub(crate) fn unsorted(&self) -> bool {
        self.unsorted
    }

    /// Notes that the steps must be sorted again before the next draw: a
    /// frame's shift or clip changed, or paint order was walked again.
    pub(crate) fn unsort(&mut self) {
        (self.unsorted, self.stale) = (true, true);
    }

    /// True when the batches are kept for the next draw.
    #[cfg(test)]
    pub(crate) fn holds_batches(&self) -> bool {
        self.current().is_some()
    }

    /// The labels and steps shown.
    #[cfg(test)]
    pub(crate) fn shown(&self) -> Vec<(Label, Step)> {
        self.shown
            .iter()
            .map(|shown| (shown.label, shown.step))
            .collect()
    }

    /// Each framed lane's rows' slots, for batching the scene afresh as
    /// these were batched (the debug check, `Painted::fresh_batches`).
    pub(crate) fn slots(&self) -> &RowSlots {
        &self.slots
    }

    /// The batches made at the last draw, when no shown step changed
    /// since.
    pub(crate) fn current(&self) -> Option<&KeptBatched> {
        self.batched.as_ref().filter(|_| !self.stale)
    }

    /// The batches, made from the shown steps' items when none are kept
    /// or a shown step changed.
    pub(crate) fn batches(&mut self, sources: Sources<'_>) -> &KeptBatched {
        if self.current().is_none() {
            let (shown, boxes) = (&self.shown, (sources.boxes, sources.frames));
            let mut lane = crate::framed_batches::LastSeen::default();
            let rows = shown.iter().filter_map(|shown| {
                let container = lane.get(shown.frame, || {
                    sources.frames.lane_container_of(shown.frame)
                });
                Some((shown.frame, container?))
            });
            self.slots.keep_only(rows);
            let slots = (self.view, &mut self.slots);
            let mut framed = framed_batches(shown, |shown| shown.step, boxes, slots);
            self.shown_culled = framed.batched.culled;
            framed.batched.culled = self.culled(sources.extents);
            let was = self.batched.as_ref();
            self.batched = Some(KeptBatched::keyed(framed, was));
            self.stale = false;
        }
        self.batched.as_ref().expect("made above")
    }

    /// The items culled: every drawing step's in `extents` but those of
    /// the shown steps not culled when the batches were made.
    fn culled(&self, extents: &Extents) -> u64 {
        let total = extents.items() + self.shown_culled;
        total.checked_sub(self.shown_items).unwrap_or_else(|| {
            panic!(
                "the kept batches show {} items of {} drawn: a shown step was counted twice or \
                 the extents index lost a step (kept_batches.rs, frame_extents.rs); reproduce \
                 with `cargo xtask test`",
                self.shown_items,
                extents.items()
            )
        })
    }
}

#[path = "kept_batches_patch.rs"]
mod patch;
#[path = "kept_batches_search.rs"]
mod search;

#[cfg(test)]
#[path = "kept_batches_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "kept_batches_draw_tests.rs"]
mod draw_tests;

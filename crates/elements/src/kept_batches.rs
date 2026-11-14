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
use crate::frame_extents::Extents;
use crate::framed_batches::{framed_batches, hidden_items};
use crate::frames::Frames;
use crate::kept_batches_keys::KeptBatched;
use crate::kept_box::KeptBox;
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

/// A shown step: its place, the step, and how many items it paints.
pub(crate) type Shown = (usize, Step, u64);

/// The shown steps and batches for one view.
#[derive(Debug)]
pub(crate) struct KeptBatches {
    pub(crate) view: Rect,
    /// The drawing steps with an item in `view` or a surface, ordered by
    /// place.
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
}

/// The frames whose steps can show in `view`: those whose container's
/// clip meets it (`frames_nested.rs`), and those holding a step that
/// spans every view, which a culled surface's batch needs; each once.
fn frames_to_search(view: Rect, sources: Sources<'_>) -> Vec<usize> {
    let mut frames = sources.frames.meeting(view);
    frames.extend(sources.extents.spanning_frames());
    frames.sort_unstable();
    frames.dedup();
    frames
}

impl KeptBatches {
    /// The steps of `sources` that show in `view`: in each frame that
    /// can show in it, the extents index's candidates for the view as it
    /// falls in the frame, tested item by item.
    pub(crate) fn of(view: Rect, sources: Sources<'_>) -> Self {
        #[cfg(test)]
        BUILDS.with(|builds| builds.set(builds.get() + 1));
        let mut shown = Vec::new();
        let boxes = (sources.boxes, sources.frames);
        // Only the frames whose container's clip meets the view are
        // searched (`frames_nested.rs`): O(shown frames · log steps).
        for frame in frames_to_search(view, sources) {
            let in_frame = sources.frames.view_in(frame, view);
            let candidates = sources.extents.candidates(frame, in_frame);
            let showing = candidates.filter(|e| hidden_items(e.step, boxes, view).is_none());
            shown.extend(showing.map(|e| (e.place, e.step, e.items)));
        }
        shown.sort_unstable_by_key(|&(place, ..)| place);
        #[cfg(debug_assertions)]
        crate::kept_batches_check::assert_candidates(&shown, view, sources);
        let shown_items = shown.iter().map(|&(.., items)| items).sum();
        Self {
            view,
            shown,
            shown_items,
            shown_culled: 0,
            batched: None,
            stale: true,
            unsorted: false,
        }
    }

    /// `slot`'s kept batches for `view`, sorted from `sources` when it
    /// holds none, holds them for another view or unsorted; with their
    /// batches made, keeping the keys of equal runs of the last ones.
    pub(crate) fn for_view(slot: &mut Option<Self>, view: Rect, sources: Sources<'_>) {
        if slot
            .as_ref()
            .is_some_and(|kept| kept.view != view || kept.unsorted)
        {
            let was = slot.take().and_then(|kept| kept.batched);
            *slot = Some(Self {
                batched: was,
                ..Self::of(view, sources)
            });
        }
        slot.get_or_insert_with(|| Self::of(view, sources))
            .batches(sources);
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

    /// The places and steps shown.
    #[cfg(test)]
    pub(crate) fn shown(&self) -> Vec<(usize, Step)> {
        self.shown
            .iter()
            .map(|&(place, step, _)| (place, step))
            .collect()
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
            let (mut batched, frames) =
                framed_batches(shown, |&(_, step, _)| step, boxes, self.view);
            self.shown_culled = batched.culled;
            batched.culled = self.culled(sources.extents);
            let was = self.batched.as_ref();
            self.batched = Some(KeptBatched::keyed((batched, frames), was));
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

#[cfg(test)]
#[path = "kept_batches_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "kept_batches_draw_tests.rs"]
mod draw_tests;

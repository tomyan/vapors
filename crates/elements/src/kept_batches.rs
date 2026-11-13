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

/// The shown steps, hidden counts and batches for one view.
#[derive(Debug)]
pub(crate) struct KeptBatches {
    pub(crate) view: Rect,
    /// The drawing steps with an item in `view` or a surface, each with
    /// its place, ordered by place.
    shown: Vec<(usize, Step)>,
    /// By place: the items of a drawing step not in `shown`, all culled.
    hidden: Vec<u64>,
    /// The sum of `hidden`.
    hidden_culled: u64,
    /// The batches of `shown`'s items, `hidden_culled` added to their
    /// culled count, with their keys; `None` until the first draw.
    batched: Option<KeptBatched>,
    /// True when a shown step changed since `batched` was made: the
    /// next draw makes them again, keeping the keys of equal runs.
    stale: bool,
    /// True when `shown` and `hidden` no longer hold: a frame's shift or
    /// clip changed, or paint order was walked. The next draw sorts the
    /// steps again, keeping `batched` for its keys.
    unsorted: bool,
}

impl KeptBatches {
    /// The steps of `drawn`, `places` steps long in all, sorted by
    /// whether they show in `view`.
    pub(crate) fn of(
        view: Rect,
        (drawn, places): (&DrawnSteps, usize),
        boxes: (&[Option<KeptBox>], &Frames),
    ) -> Self {
        #[cfg(test)]
        BUILDS.with(|builds| builds.set(builds.get() + 1));
        let mut kept = Self {
            view,
            shown: Vec::new(),
            hidden: vec![0; places],
            hidden_culled: 0,
            batched: None,
            stale: false,
            unsorted: false,
        };
        for (place, step) in drawn.placed() {
            kept.add(place, step, boxes);
        }
        kept
    }

    /// `slot`'s kept batches for `view`, sorted from `drawn` when it
    /// holds none, holds them for another view or unsorted; with their
    /// batches made, keeping the keys of equal runs of the last ones.
    pub(crate) fn for_view(
        slot: &mut Option<Self>,
        view: Rect,
        drawn: (&DrawnSteps, usize),
        boxes: (&[Option<KeptBox>], &Frames),
    ) {
        if slot
            .as_ref()
            .is_some_and(|kept| kept.view != view || kept.unsorted)
        {
            let was = slot.take().and_then(|kept| kept.batched);
            let sorted = Self::of(view, drawn, boxes);
            *slot = Some(Self {
                batched: was,
                stale: true,
                ..sorted
            });
        }
        slot.get_or_insert_with(|| Self::of(view, drawn, boxes))
            .batches(boxes);
    }

    /// Notes that the steps must be sorted again before the next draw: a
    /// frame's shift or clip changed, or paint order was walked again.
    pub(crate) fn unsort(&mut self) {
        (self.unsorted, self.stale) = (true, true);
    }

    /// `self` with `steps`, each with its place, sorted again from their
    /// parts in `boxes`; nothing when every step is sorted again at the
    /// next draw.
    pub(crate) fn patch(
        &mut self,
        steps: impl Iterator<Item = (usize, Step)>,
        boxes: (&[Option<KeptBox>], &Frames),
    ) {
        if self.unsorted {
            return;
        }
        for (place, step) in steps {
            self.remove(place);
            if boxes.0.get(step.index()).is_some_and(Option::is_some) {
                self.add(place, step, boxes);
            }
        }
    }

    /// True when the batches are kept for the next draw.
    #[cfg(test)]
    pub(crate) fn holds_batches(&self) -> bool {
        self.current().is_some()
    }

    /// The batches made at the last draw, when no shown step changed
    /// since.
    pub(crate) fn current(&self) -> Option<&KeptBatched> {
        self.batched.as_ref().filter(|_| !self.stale)
    }

    /// The batches, made from the shown steps' items when none are kept
    /// or a shown step changed.
    pub(crate) fn batches(&mut self, boxes: (&[Option<KeptBox>], &Frames)) -> &KeptBatched {
        if self.current().is_none() {
            let shown = &self.shown;
            let (mut batched, frames) = framed_batches(shown, |&(_, step)| step, boxes, self.view);
            batched.culled += self.hidden_culled;
            let was = self.batched.as_ref();
            self.batched = Some(KeptBatched::keyed((batched, frames), was));
            self.stale = false;
        }
        self.batched.as_ref().expect("made above")
    }

    /// Puts `step`, at `place`, in `shown` or `hidden` by its items.
    fn add(&mut self, place: usize, step: Step, boxes: (&[Option<KeptBox>], &Frames)) {
        match hidden_items(step, boxes, self.view) {
            None => {
                let at = self.shown.partition_point(|&(shown, _)| shown < place);
                self.shown.insert(at, (place, step));
                self.stale = true;
            }
            Some(culled) => {
                self.hidden[place] = culled;
                self.add_culled(culled.cast_signed());
            }
        }
    }

    /// Takes the step at `place` out of `shown` or `hidden`.
    fn remove(&mut self, place: usize) {
        if let Ok(at) = self.shown.binary_search_by_key(&place, |&(shown, _)| shown) {
            self.shown.remove(at);
            self.stale = true;
        }
        let culled = std::mem::take(&mut self.hidden[place]);
        self.add_culled(-culled.cast_signed());
    }

    /// Adds `culled` hidden items to the sum and to any kept batches'
    /// count.
    fn add_culled(&mut self, culled: i64) {
        self.hidden_culled = added(self.hidden_culled, culled);
        if let Some(kept) = &mut self.batched {
            kept.batched.culled = added(kept.batched.culled, culled);
        }
    }
}

/// `count` plus `change`, or a panic when it would go below zero: a
/// step was taken out that was never put in.
fn added(count: u64, change: i64) -> u64 {
    count.checked_add_signed(change).unwrap_or_else(|| {
        panic!(
            "kept batches culled count {count} would go below zero by {change}: a step was \
             taken out of them twice (kept_batches.rs); reproduce with `cargo xtask test`"
        )
    })
}

#[cfg(test)]
#[path = "kept_batches_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "kept_batches_draw_tests.rs"]
mod draw_tests;

//! Batches kept across draws (IU3 3c-3f, `docs/plans/iu3.md` "3c-3f:
//! kept batches"): for the view of the last draw, the drawing steps
//! that show something in it, the culled count of the others, and the
//! batches of the shown steps' items. A draw with the same view reads
//! the kept batches; an update patches the steps of the slots it paints
//! again (`paint_partial.rs`). A debug build compares every draw's
//! batches with [`vapors_scene::batched`] over the whole scene.

use vapors_scene::{Batched, Item, Rect};

use crate::drawn_steps::DrawnSteps;
use crate::frames::Frames;
use crate::kept_box::KeptBox;
use crate::paint_order::Step;
use crate::scene_view::step_items;

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
    /// culled count; `None` until the next draw after a shown step changed.
    pub(crate) batched: Option<Batched>,
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
        };
        for (place, step) in drawn.placed() {
            kept.add(place, step, boxes);
        }
        kept
    }

    /// `slot`'s kept batches for `view`, made from `drawn` when it holds
    /// none or holds them for another view; with their batches made.
    pub(crate) fn for_view(
        slot: &mut Option<Self>,
        view: Rect,
        drawn: (&DrawnSteps, usize),
        boxes: (&[Option<KeptBox>], &Frames),
    ) {
        if slot.as_ref().is_some_and(|kept| kept.view != view) {
            *slot = None;
        }
        slot.get_or_insert_with(|| Self::of(view, drawn, boxes))
            .batches(boxes);
    }

    /// `self` with `steps`, each with its place, sorted again from their
    /// parts in `boxes`.
    pub(crate) fn patch(
        &mut self,
        steps: impl Iterator<Item = (usize, Step)>,
        boxes: (&[Option<KeptBox>], &Frames),
    ) {
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
        self.batched.is_some()
    }

    /// The batches, made from the shown steps' items when none are kept.
    pub(crate) fn batches(&mut self, boxes: (&[Option<KeptBox>], &Frames)) -> &Batched {
        self.batched.get_or_insert_with(|| {
            let items = self
                .shown
                .iter()
                .flat_map(|&(_, step)| step_items(boxes, step));
            let mut batched = vapors_scene::batched(items, self.view);
            batched.culled += self.hidden_culled;
            batched
        })
    }

    /// Puts `step`, at `place`, in `shown` or `hidden` by its items.
    fn add(&mut self, place: usize, step: Step, boxes: (&[Option<KeptBox>], &Frames)) {
        match hidden_items(step_items(boxes, step), self.view) {
            None => {
                let at = self.shown.partition_point(|&(shown, _)| shown < place);
                self.shown.insert(at, (place, step));
                self.batched = None;
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
            self.batched = None;
        }
        let culled = std::mem::take(&mut self.hidden[place]);
        self.add_culled(-culled.cast_signed());
    }

    /// Adds `culled` hidden items to the sum and to any kept batches'
    /// count.
    fn add_culled(&mut self, culled: i64) {
        self.hidden_culled = added(self.hidden_culled, culled);
        if let Some(batched) = &mut self.batched {
            batched.culled = added(batched.culled, culled);
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

/// How many `items` there are when every one is culled in `view` and
/// none is a surface (a culled surface keeps its place in a batch);
/// `None` when one shows.
fn hidden_items(items: impl Iterator<Item = (Item, Option<Rect>)>, view: Rect) -> Option<u64> {
    let mut count = 0;
    for (item, clip) in items {
        let surface = matches!(item, Item::Surface { .. });
        if surface || !vapors_scene::culled(&item, clip, view) {
            return None;
        }
        count += 1;
    }
    Some(count)
}

#[cfg(test)]
#[path = "kept_batches_tests.rs"]
mod tests;

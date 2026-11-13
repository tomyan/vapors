//! Patching kept batches (`kept_batches.rs`) for the steps an update
//! painted again, shifted or freed: each is taken out of the shown
//! steps and put back when one of its items shows in the view.

use super::{KeptBatches, Sources};
use crate::framed_batches::hidden_items;
use crate::paint_order::Step;

impl KeptBatches {
    /// `self` with `steps`, each with its place, sorted again from their
    /// parts in `sources`; nothing when every step is sorted again at
    /// the next draw.
    pub(crate) fn patch(
        &mut self,
        steps: impl Iterator<Item = (usize, Step)>,
        sources: Sources<'_>,
    ) {
        if self.unsorted {
            return;
        }
        for (place, step) in steps {
            self.remove(place);
            if sources.boxes.get(step.index()).is_some_and(Option::is_some) {
                self.add(place, step, sources);
            }
        }
        let culled = self.culled(sources.extents);
        if let Some(kept) = &mut self.batched {
            kept.batched.culled = culled;
        }
    }

    /// Puts `step`, at `place`, in `shown` when one of its items shows,
    /// with the item count its entry in the extents index has.
    fn add(&mut self, place: usize, step: Step, sources: Sources<'_>) {
        if hidden_items(step, (sources.boxes, sources.frames), self.view).is_some() {
            return;
        }
        let items = sources.extents.items_at(place);
        let at = self.shown.partition_point(|&(shown, ..)| shown < place);
        self.shown.insert(at, (place, step, items));
        self.shown_items += items;
        self.stale = true;
    }

    /// Takes the step at `place` out of `shown`, if there.
    fn remove(&mut self, place: usize) {
        if let Ok(at) = self
            .shown
            .binary_search_by_key(&place, |&(shown, ..)| shown)
        {
            self.shown_items -= self.shown.remove(at).2;
            self.stale = true;
        }
    }
}

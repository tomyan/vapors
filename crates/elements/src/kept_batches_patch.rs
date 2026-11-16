//! Patching kept batches (`kept_batches.rs`) for the steps an update
//! painted again, shifted or freed: each is taken out of the shown
//! steps and put back when one of its items shows in the view.

use super::{KeptBatches, Shown, Sources};
use crate::framed_batches::hidden_items;
use crate::kept_order_splice::SpliceEdits;
use crate::paint_labels::Label;
use crate::paint_order::Step;

impl KeptBatches {
    /// `self` with `steps`, each with its label, sorted again from their
    /// parts in `sources`; nothing when every step is sorted again at
    /// the next draw.
    pub(crate) fn patch(
        &mut self,
        steps: impl Iterator<Item = (Label, Step)>,
        sources: Sources<'_>,
    ) {
        if self.unsorted {
            return;
        }
        for (label, step) in steps {
            _ = self.remove(label);
            if sources.boxes.get(step.index()).is_some_and(Option::is_some) {
                self.add(label, step, sources);
            }
        }
        let culled = self.culled(sources.extents);
        if let Some(kept) = &mut self.batched {
            kept.batched.culled = culled;
        }
    }

    /// Puts `step`, at `label`, in `shown` when one of its items shows,
    /// with the item count its entry in the extents index has.
    fn add(&mut self, label: Label, step: Step, sources: Sources<'_>) {
        if hidden_items(step, (sources.boxes, sources.frames), self.view).is_some() {
            return;
        }
        let items = sources.extents.items_at(label, step);
        let frame = crate::scene_view::frame_items(sources.boxes, step).0;
        self.insert_shown(Shown {
            label,
            step,
            items,
            frame,
        });
    }

    /// `shown` put among the shown steps by its label.
    ///
    /// # Panics
    ///
    /// When a step is shown at that label already: two steps at one label
    /// would leave one undrawn, or one drawn twice.
    pub(super) fn insert_shown(&mut self, shown: Shown) {
        let at = self.shown.partition_point(|kept| kept.label < shown.label);
        if let Some(kept) = self.shown.get(at).filter(|kept| kept.label == shown.label) {
            panic!(
                "step {:?} is put among the shown steps at label {}, which step {:?} holds: a \
                 relabel moved a step onto a label not yet given up, or a step was put in twice \
                 (kept_batches_patch.rs, kept_batches_search.rs); reproduce with `cargo xtask test`",
                shown.step, shown.label, kept.step
            );
        }
        self.shown.insert(at, shown);
        self.shown_items += shown.items;
        self.stale = true;
    }

    /// The shown steps `keep` refuses taken out.
    pub(super) fn retain_shown(&mut self, keep: impl Fn(&Shown) -> bool) {
        let before = self.shown.len();
        self.shown.retain(|shown| keep(shown));
        self.shown_items = self.shown.iter().map(|shown| shown.items).sum();
        self.stale |= self.shown.len() != before;
    }

    /// A splice's `edits` applied to the shown steps: the steps taken
    /// out removed (a freed slot is not always among those the update
    /// patches), those relabelled moved to their new labels; every step
    /// sorted again at the next draw when the splice relabelled every
    /// label. The steps put in are those of slots painted, which the
    /// update patches (`patch`).
    pub(crate) fn spliced(&mut self, edits: &SpliceEdits) {
        if self.unsorted {
            return;
        }
        if edits.all {
            self.unsort();
            return;
        }
        for &(label, _) in &edits.removed {
            _ = self.remove(label);
        }
        // every old label given up before any new one is taken: a step can
        // move onto a label another step is moving off
        let moved: Vec<Shown> = edits
            .relabelled
            .iter()
            .filter_map(|&(_, old, new)| {
                Some(Shown {
                    label: new,
                    ..self.remove(old)?
                })
            })
            .collect();
        for shown in moved {
            self.insert_shown(shown);
        }
    }

    /// Takes the step at `label` out of `shown`, if there: the step.
    fn remove(&mut self, label: Label) -> Option<Shown> {
        let at = self
            .shown
            .binary_search_by_key(&label, |shown| shown.label)
            .ok()?;
        let shown = self.shown.remove(at);
        self.shown_items -= shown.items;
        self.stale = true;
        Some(shown)
    }
}

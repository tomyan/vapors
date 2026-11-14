//! The paint steps that draw something, in paint order (review v201),
//! each by its label (`paint_labels.rs`):
//! batching walks only these, so it reads neither the steps that paint
//! nothing nor their slots' kept paint. Made afresh when paint order is
//! walked again; else patched for the slots painted again or freed. A
//! debug build compares it with one made afresh (`kept_paint_check.rs`).
//! The drawing steps are kept in blocks (`blocks.rs`), so a step put
//! in or taken out moves at most a block's tail, not the page's.

use crate::blocks::{Blocks, IndexCounts};
use crate::kept_box::KeptBox;
use crate::kept_order_splice::SpliceEdits;
use crate::paint_labels::Label;
use crate::paint_order::Step;

/// Each slot's steps' labels in paint order (`paint_labels.rs`), and the
/// steps whose items are not empty, each with its label.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct DrawnSteps {
    /// By slot: the labels of its box, thumb and outline steps.
    labels: Vec<[Option<Label>; 3]>,
    /// The steps that draw, ordered by label.
    drawn: Blocks<(Label, Step)>,
}

impl DrawnSteps {
    /// The steps of `steps`, labelled `labels`, whose part in `boxes`
    /// holds items.
    pub(crate) fn of((steps, labels): (&[Step], &[Label]), boxes: &[Option<KeptBox>]) -> Self {
        let mut by_slot = vec![[None; 3]; boxes.len()];
        for (&label, &step) in labels.iter().zip(steps) {
            if let Some(slot) = by_slot.get_mut(step.index()) {
                slot[kind(step)] = Some(label);
            }
        }
        let labelled = labels.iter().copied().zip(steps.iter().copied());
        let drawn = labelled.filter(|&(_, step)| draws(boxes, step)).collect();
        Self {
            labels: by_slot,
            drawn: Blocks::from_sorted(drawn),
        }
    }

    /// `self` with the steps of each of `slots` put in or taken out as
    /// its part in `boxes` now holds items or not.
    pub(crate) fn patch(&mut self, slots: impl Iterator<Item = usize>, boxes: &[Option<KeptBox>]) {
        for slot in slots {
            let labelled: Vec<(Label, Step)> = self.labels_of(slot).collect();
            for (label, step) in labelled {
                self.patch_step(label, step, boxes);
            }
        }
    }

    /// `self` following a splice's `edits` (`kept_order_labelled.rs`), for
    /// `slots` slots: the steps taken out gone, those relabelled under
    /// their new labels (in place: a relabel keeps the order), those put
    /// in labelled, to be put in as they draw by the patch for their
    /// slots that follows. O(the edits' steps, log the drawn steps).
    pub(crate) fn splice(&mut self, edits: &SpliceEdits, slots: usize) {
        self.labels.resize(slots, [None; 3]);
        for &(label, step) in &edits.removed {
            self.forget(label, step);
        }
        self.relabel(&edits.relabelled);
        for &(label, step) in &edits.inserted {
            self.labels[step.index()][kind(step)] = Some(label);
        }
    }

    /// Takes `step`, labelled `label`, out: its slot's label and, if it
    /// draws, its entry.
    fn forget(&mut self, label: Label, step: Step) {
        let held = &mut self.labels[step.index()][kind(step)];
        if *held == Some(label) {
            *held = None;
        }
        if let Ok(at) = self.drawn.binary_search_by(|&(at, _)| at.cmp(&label)) {
            self.drawn.remove(at);
        }
    }

    /// Each `(step, old, new)` of `relabelled` relabelled, in its slot and
    /// its entry: every entry found by its old label first, while they are
    /// in order, then given its new one (a relabel keeps the order, but
    /// one at a time would leave them out of order between).
    fn relabel(&mut self, relabelled: &[(Step, Label, Label)]) {
        let found: Vec<_> = relabelled
            .iter()
            .map(|&(_, old, _)| self.drawn.binary_search_by(|&(at, _)| at.cmp(&old)).ok())
            .collect();
        for (&(step, old, new), at) in relabelled.iter().zip(found) {
            let held = &mut self.labels[step.index()][kind(step)];
            assert_eq!(
                *held,
                Some(old),
                "{step:?} relabelled from {old} holds {held:?} (kept_order_labelled.rs)"
            );
            *held = Some(new);
            if let Some(at) = at {
                self.drawn.get_mut(at).expect("found").0 = new;
            }
        }
    }

    /// Puts in or takes out `step`, labelled `label`, as it draws or not.
    fn patch_step(&mut self, label: Label, step: Step, boxes: &[Option<KeptBox>]) {
        let found = self.drawn.binary_search_by(|&(at, _)| at.cmp(&label));
        match (found, draws(boxes, step)) {
            (Err(at), true) => _ = self.drawn.insert(at, (label, step)),
            (Ok(at), false) => _ = self.drawn.remove(at),
            _ => {}
        }
    }

    /// The entries the patches since the last take moved and the blocks
    /// held (`blocks.rs` `IndexCounts`), for `UpdateCounts`.
    pub(crate) fn take_counts(&mut self) -> IndexCounts {
        IndexCounts {
            moved: self.drawn.take_moved(),
            rebuilds: 0,
            blocks: self.drawn.block_count(),
        }
    }

    /// The steps that draw, in paint order.
    pub(crate) fn steps(&self) -> impl Iterator<Item = Step> + '_ {
        self.drawn.iter().map(|&(_, step)| step)
    }

    /// The steps that draw, each with its label, in paint order.
    pub(crate) fn labelled(&self) -> impl Iterator<Item = (Label, Step)> + '_ {
        self.drawn.iter().copied()
    }

    /// The label of `step` in paint order, if it is in it.
    pub(crate) fn label_of(&self, step: Step) -> Option<Label> {
        self.labels.get(step.index())?[kind(step)]
    }

    /// Slot `slot`'s steps that are in paint order, each with its label.
    pub(crate) fn labels_of(&self, slot: usize) -> impl Iterator<Item = (Label, Step)> {
        let labels = self.labels.get(slot).copied().unwrap_or_default();
        let steps = labels.into_iter().zip(steps_of(slot));
        steps.filter_map(|(label, step)| Some((label?, step)))
    }
}

/// True when `step`'s slot holds a kept paint whose items for `step`
/// are not empty.
pub(crate) fn draws(boxes: &[Option<KeptBox>], step: Step) -> bool {
    let kept = boxes.get(step.index()).and_then(Option::as_ref);
    kept.is_some_and(|kept| !kept.scene_of(step).items().is_empty())
}

/// The index of `step`'s kind in a slot's labels.
pub(crate) fn kind(step: Step) -> usize {
    match step {
        Step::Box(_) => 0,
        Step::Thumb(_) => 1,
        Step::Outline(_) => 2,
    }
}

/// Slot `slot`'s steps, in the order of [`kind`].
fn steps_of(slot: usize) -> [Step; 3] {
    [Step::Box(slot), Step::Thumb(slot), Step::Outline(slot)]
}

#[cfg(test)]
#[path = "drawn_steps_tests.rs"]
mod tests;

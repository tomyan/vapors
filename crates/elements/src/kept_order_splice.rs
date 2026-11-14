//! The kept order spliced (`order_splice.rs`): the steps edited, the
//! new slots put below their parents and the freed ones taken out of
//! the clip tree and clips, the new elements joined to their parent's
//! scroll chain and the freed ones left, then the clips redone under
//! the changed slots as on an update that kept the shape.

use std::ops::Range;

use crate::changed_slots::ChangedSlots;
use crate::kept_order::KeptOrder;
use crate::layout::{BoxRect, Content, Key, LayoutBox};
use crate::order_splice::{Edit, Splice};
use crate::paint::PaintInput;
use crate::paint_labels::Label;
use crate::paint_order::Step;
use labelled::Labelled;
pub(crate) use labelled::SpliceEdits;

impl KeptOrder {
    /// `self` with `splice` made, for the `changed` slots of `boxes`.
    pub(crate) fn spliced(
        mut self,
        splice: Splice,
        (boxes, rects): (&[LayoutBox], &[BoxRect]),
        input: &PaintInput<'_>,
        changed: &ChangedSlots,
    ) -> Self {
        let (steps, labels, runs, removed) =
            spliced_steps((&self.steps, &self.labels), splice.edits);
        let recent = std::mem::take(&mut self.recent);
        let labelled = Labelled::of((&steps, labels), &runs, (&mut self.spacing, &recent));
        (self.labels, self.recent) = (labelled.labels, labelled.put_in);
        self.steps = steps;
        let edits = SpliceEdits {
            removed,
            ..labelled.edits
        };
        let freed = || changed.freed.iter().map(|&(slot, _)| slot);
        self.tree.spliced(boxes.len(), &splice.added, freed());
        self.clips.spliced(boxes.len(), freed());
        self.chains_spliced(boxes, &splice.added, &changed.freed);
        let mut order = self.reclipped((boxes, rects), input, changed);
        order.walks.splices = 1;
        order.walks.relabelled = edits.relabelled.len();
        order.walks.relabel_all = usize::from(edits.all);
        order.splice_edits = edits;
        order
    }

    /// Each `freed` element left its scroll chain; each `added`
    /// element, under its parent, joined the parent's.
    fn chains_spliced(
        &mut self,
        boxes: &[LayoutBox],
        added: &[(usize, usize)],
        freed: &[(usize, Key)],
    ) {
        for (_, key) in freed {
            if let Key::Element(node) = key {
                self.chains.left(*node);
            }
        }
        for &(slot, parent) in added {
            if let (Content::Element(node), Content::Element(above)) =
                (&boxes[slot].content, &boxes[parent].content)
            {
                self.chains.joined(*node, *above);
            }
        }
    }
}

/// The steps and labels `spliced_steps` makes, the runs of positions the
/// inserted steps took (unlabelled, 0, until `Labelled::of`), and the
/// steps taken out with their labels.
type Spliced = (Vec<Step>, Vec<Label>, Vec<Range<usize>>, Vec<(Label, Step)>);

/// `steps` and their `labels` with `edits` made, each at its place in
/// `steps`. At one place an insertion comes before a removal, so steps
/// put in where a removed run starts are not taken out with it.
fn spliced_steps((steps, labels): (&[Step], &[Label]), mut edits: Vec<Edit>) -> Spliced {
    edits.sort_by_key(|edit| (edit.at, edit.removed));
    let grown: usize = edits.iter().map(|edit| edit.inserted.len()).sum();
    let mut out_steps = Vec::with_capacity(steps.len() + grown);
    let mut out_labels = Vec::with_capacity(steps.len() + grown);
    let mut runs: Vec<Range<usize>> = Vec::new();
    let mut removed = Vec::new();
    let mut from = 0;
    for edit in edits {
        assert!(
            edit.at >= from,
            "order splice edits overlap at step {}: two runs share a place (order_splice.rs \
             anchors); reproduce with `cargo xtask test`",
            edit.at
        );
        out_steps.extend_from_slice(&steps[from..edit.at]);
        out_labels.extend_from_slice(&labels[from..edit.at]);
        let at = out_steps.len();
        note_run(&mut runs, at..at + edit.inserted.len());
        out_labels.extend(std::iter::repeat_n(0, edit.inserted.len()));
        out_steps.extend(edit.inserted);
        let taken = edit.at..edit.at + edit.removed;
        removed.extend(
            labels[taken.clone()]
                .iter()
                .copied()
                .zip(steps[taken].iter().copied()),
        );
        from = edit.at + edit.removed;
    }
    out_steps.extend_from_slice(&steps[from..]);
    out_labels.extend_from_slice(&labels[from..]);
    (out_steps, out_labels, runs, removed)
}

/// Notes the run `run` of inserted positions, joined to the last when they
/// touch, so every run's neighbours are labelled.
fn note_run(runs: &mut Vec<Range<usize>>, run: Range<usize>) {
    if run.is_empty() {
        return;
    }
    match runs.last_mut() {
        Some(last) if last.end == run.start => last.end = run.end,
        _ => runs.push(run),
    }
}

#[path = "kept_order_labelled.rs"]
mod labelled;

#[cfg(test)]
#[path = "kept_order_splice_tests.rs"]
mod tests;

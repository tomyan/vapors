//! The kept order spliced (`order_splice.rs`): the steps edited, the
//! new slots put below their parents and the freed ones taken out of
//! the clip tree and clips, the new elements joined to their parent's
//! scroll chain and the freed ones left, then the clips redone under
//! the changed slots as on an update that kept the shape.

use crate::changed_slots::ChangedSlots;
use crate::kept_order::KeptOrder;
use crate::layout::{BoxRect, Content, Key, LayoutBox};
use crate::order_splice::{Edit, Splice};
use crate::paint::PaintInput;
use crate::paint_order::Step;

impl KeptOrder {
    /// `self` with `splice` made, for the `changed` slots of `boxes`.
    pub(crate) fn spliced(
        mut self,
        splice: Splice,
        (boxes, rects): (&[LayoutBox], &[BoxRect]),
        input: &PaintInput<'_>,
        changed: &ChangedSlots,
    ) -> Self {
        self.steps = spliced_steps(&self.steps, splice.edits);
        let freed = || changed.freed.iter().map(|&(slot, _)| slot);
        self.tree.spliced(boxes.len(), &splice.added, freed());
        self.clips.spliced(boxes.len(), freed());
        self.chains_spliced(boxes, &splice.added, &changed.freed);
        let mut order = self.reclipped((boxes, rects), input, changed);
        order.walks.splices = 1;
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

/// `steps` with `edits` made, each at its place in `steps`. At one
/// place an insertion comes before a removal, so steps put in where a
/// removed run starts are not taken out with it.
fn spliced_steps(steps: &[Step], mut edits: Vec<Edit>) -> Vec<Step> {
    edits.sort_by_key(|edit| (edit.at, edit.removed));
    let grown: usize = edits.iter().map(|edit| edit.inserted.len()).sum();
    let mut out = Vec::with_capacity(steps.len() + grown);
    let mut from = 0;
    for edit in edits {
        assert!(
            edit.at >= from,
            "order splice edits overlap at step {}: two runs share a place (order_splice.rs \
             anchors); reproduce with `cargo xtask test`",
            edit.at
        );
        out.extend_from_slice(&steps[from..edit.at]);
        out.extend(edit.inserted);
        from = edit.at + edit.removed;
    }
    out.extend_from_slice(&steps[from..]);
    out
}

#[cfg(test)]
#[path = "kept_order_splice_tests.rs"]
mod tests;

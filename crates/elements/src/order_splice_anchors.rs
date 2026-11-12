//! Where an inserted run's steps go in the kept order
//! (`order_splice.rs`): its box steps into its parent's flow run and
//! its outline steps into the enclosing frame's outlines, at the
//! matching place. The anchor is a kept in-flow sibling's step, found
//! through the slot-to-step index, or the last in-flow box of the
//! previous sibling's subtree, found by walking that subtree.

use std::ops::Range;

use super::{Children, Declined, Place, placed};
use crate::layout::LayoutBox;
use crate::paint_order::{Stackings, Step};

impl Children<'_> {
    /// Where `run`'s box steps and outline steps go: before the next
    /// kept in-flow sibling's, after the last in-flow box of the
    /// previous one's, or, when the owner keeps no children, after its
    /// own; `Declined` when it keeps only out-of-flow children, or the
    /// previous sibling holds a scroll container in flow.
    pub(super) fn anchors(
        &self,
        boxes: &[LayoutBox],
        stackings: &Stackings,
        run: &Range<usize>,
        place: Place<'_>,
    ) -> Result<(usize, usize), Declined> {
        let in_flow = |slot: &usize| !stackings.get(*slot).positioned;
        let mut after = self.new[run.end..].iter().filter(|slot| self.kept(**slot));
        if let Some(&next) = after.find(|slot| in_flow(slot)) {
            return Ok((
                placed(place, Step::Box(next)),
                placed(place, Step::Outline(next)),
            ));
        }
        let mut before = self.new[..run.start]
            .iter()
            .rev()
            .filter(|slot| self.kept(**slot));
        if let Some(&prev) = before.find(|slot| in_flow(slot)) {
            let last = last_in_flow(boxes, stackings, prev)?;
            return Ok((
                placed(place, Step::Box(last)) + 1,
                placed(place, Step::Outline(last)) + 1,
            ));
        }
        if self.new.iter().any(|slot| self.kept(*slot)) {
            return Err(Declined::OnlyOutOfFlowKept);
        }
        let own = self.owner;
        Ok((
            placed(place, Step::Box(own)) + 1,
            placed(place, Step::Outline(own)) + 1,
        ))
    }
}

/// The last box painted in `root`'s flow: the last of `root` and its
/// in-flow descendants in tree order; `Declined` when one scrolls, as
/// its thumb's place is not read.
fn last_in_flow(
    boxes: &[LayoutBox],
    stackings: &Stackings,
    root: usize,
) -> Result<usize, Declined> {
    let (mut last, mut open) = (root, vec![root]);
    while let Some(slot) = open.pop() {
        let stacking = stackings.get(slot);
        if stacking.positioned {
            continue;
        }
        if stacking.scrolls {
            return Err(Declined::ScrollInPreviousSibling);
        }
        last = slot;
        open.extend(boxes[slot].children.iter().rev());
    }
    Ok(last)
}

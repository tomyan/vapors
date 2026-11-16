//! Where an owner's edits fall among its children (`put_edited`): a
//! child's index found through the owner's lane in O(log n)
//! (`LaneTree::rank`), not by a search of the children, and the run of
//! children the edits change, derived from those indices rather than by
//! comparing the old and new lists (`reshaped.rs`).

use vapors_dom::NodeId;

use super::super::super::boxes::Source;
use super::super::super::child_edits::Edits;
use super::super::super::reshaped::{Reshaped, Shared};
use super::{Placed, Taken, position, previous_box};

impl Placed {
    /// The index of the child in `child` among `children`, the owner's
    /// in `owner`: through the owner's lane when it holds every child,
    /// else by a search. Debug builds check the two agree.
    pub(super) fn child_position(&self, owner: usize, children: &[usize], child: usize) -> usize {
        let Some(at) = self.lanes.borrow().index_of(owner, (child, children.len())) else {
            let at = position(children, child);
            self.note_scanned(at + 1);
            return at;
        };
        #[cfg(debug_assertions)]
        assert_eq!(
            at,
            position(children, child),
            "the lane of slot {owner} puts child {child} at {at}, its box's children elsewhere: \
             the lane and the box's children are out of step (layout/slots_edit.rs); \
             reproduce with `cargo xtask test`"
        );
        at
    }

    /// The run of `old`, the owner's children before `edits`, that the
    /// edits change: from the first index a removal or an insertion
    /// touches to the last, the shared head and tail outside it.
    pub(super) fn reshaped_by(
        &self,
        source: &Source<'_>,
        (owner, old, edits): (usize, &[usize], &Edits),
        taken: &Taken,
    ) -> Reshaped {
        let (mut from, mut to) = (old.len(), 0);
        for &removed in &edits.removed {
            let at = self.child_position(owner, old, removed);
            (from, to) = (from.min(at), to.max(at + 1));
        }
        for &child in &edits.inserted {
            let at = self.old_place(source, (owner, old, edits), child, taken);
            (from, to) = (from.min(at), to.max(at));
        }
        let to = to.max(from);
        Reshaped {
            slot: owner,
            shared: Shared {
                head: from,
                tail: old.len() - to,
            },
            old_between: old[from..to].to_vec(),
        }
    }

    /// Where the inserted `child` goes among `old`, the owner's children
    /// before the edits: after its nearest previous sibling with a box,
    /// or that sibling's place when it is inserted too, or first.
    fn old_place(
        &self,
        source: &Source<'_>,
        (owner, old, edits): (usize, &[usize], &Edits),
        child: NodeId,
        taken: &Taken,
    ) -> usize {
        let mut at = child;
        loop {
            let Some(previous) = previous_box(source, at) else {
                return 0;
            };
            if !edits.inserted.contains(&previous) {
                let slot = self.slot_held(previous, taken);
                return self.child_position(owner, old, slot) + 1;
            }
            at = previous;
        }
    }
}

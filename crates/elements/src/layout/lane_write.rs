//! A lane put back and its children's layouts written
//! (`lane_layout.rs`): each child from the first that moved on gets its
//! start from the tree, from where the kind measures starts.

use taffy::{LayoutPartialTree, NodeId};

use super::lane::Lane;
use super::lane_block_answer::narrow;
use super::lane_kind::LaneKind;
use super::tree::BoxTree;

impl BoxTree<'_> {
    /// Puts `lane` back among the lanes as the lane of the container in
    /// `slot`, noting its children put since.
    pub(super) fn lane_kept<K: LaneKind>(&mut self, slot: usize, mut lane: Lane<K>) {
        let born = std::mem::take(&mut lane.born);
        K::lanes(self.lanes).insert(slot, lane);
        self.lanes
            .parents
            .extend(born.into_iter().map(|child| (child, slot)));
    }

    /// Writes the layout of every child from the first that moved on:
    /// its start from the tree, from where the kind measures starts.
    pub(super) fn write_starts<K: LaneKind>(
        &mut self,
        c: &K::Container,
        lane: &mut Lane<K>,
        run: &K::Summary,
    ) {
        let Some(first) = lane.first_moved() else {
            return;
        };
        let top = K::top(c, run);
        for (handle, start) in lane.tree.starts_from(first) {
            let slot = lane.slots[&handle];
            let (mut layout, relative) = lane.take_write(slot, self.boxes[slot].layout);
            layout.location.y = narrow(top + start) + relative;
            if layout.location.y.to_bits() != self.boxes[slot].layout.location.y.to_bits() {
                self.written.started.push(slot);
            }
            self.set_unrounded_layout(NodeId::from(slot), &layout);
        }
        lane.moved.clear();
        lane.pending.clear();
    }
}

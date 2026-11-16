//! A lane put back and its children's layouts written
//! (`lane_layout.rs`): each child laid out gets its layout with its
//! relative offset alone along the axis; its start stays in the lane
//! (`docs/plans/splice-layout.md`, section 11.2).

use taffy::{LayoutPartialTree, NodeId};

use super::lane::Lane;
use super::lane_kind::LaneKind;
use super::tree::BoxTree;

impl BoxTree<'_> {
    /// Puts `lane` back among the lanes as the lane of the container in
    /// `slot`, noting its children put since.
    pub(super) fn lane_kept<K: LaneKind>(&mut self, slot: usize, mut lane: Lane<K>) {
        let born = std::mem::take(&mut lane.born);
        self.lanes.changes += 1;
        K::lanes(self.lanes).insert(slot, lane);
        self.lanes
            .parents
            .extend(born.into_iter().map(|child| (child, slot)));
    }

    /// Writes the layout of each child laid out since the last write,
    /// its `location.y` its relative offset only (section 11.2), and
    /// notes the children from the first that moved on as started.
    pub(super) fn write_laid<K: LaneKind>(&mut self, container: usize, lane: &mut Lane<K>) {
        if let Some(first) = lane.first_moved() {
            self.written.started.push((container, lane.slots[&first]));
        }
        for (slot, layout) in std::mem::take(&mut lane.pending) {
            lane.note_relative(slot, layout.location.y);
            self.set_unrounded_layout(NodeId::from(slot), &layout);
        }
        lane.moved.clear();
        #[cfg(debug_assertions)]
        lane.note_written_starts();
    }
}

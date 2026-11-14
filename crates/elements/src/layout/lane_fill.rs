//! How a block lane's leaves are made (`lane_layout.rs` routes a
//! container here): every child into a fresh tree, or only the children
//! whose leaves changed, each laid out by `lane_block_child.rs` with
//! its layout held for the lane's write.

use taffy::NodeId;

use super::lane::Lane;
use super::lane_block::Container;
use super::lane_block_summary::BlockSummary;
use super::lane_share::reads_heights;
use super::tree::BoxTree;
use crate::lane_decline::LaneDecline;

impl BoxTree<'_> {
    /// The children whose leaves changed laid out again: those reading
    /// the height bases when they changed, those put in, and those
    /// built or cleared.
    pub(super) fn fill_changed(
        &mut self,
        c: &Container,
        lane: &mut Lane,
        heights_changed: bool,
    ) -> Result<(), LaneDecline> {
        if heights_changed {
            let readers = lane.height_readers.iter().copied().collect();
            self.lay_out_again(readers, c, lane)?;
        }
        self.fill_inserted(c, lane)?;
        self.fill_stale(c, lane)
    }

    /// Every child laid out and put in a fresh tree, in order.
    pub(super) fn fill_all(
        &mut self,
        slot: usize,
        c: &Container,
        lane: &mut Lane,
    ) -> Result<(), LaneDecline> {
        *lane = Lane {
            share: lane.share.take(),
            heights: lane.heights.take(),
            all: false,
            ..Lane::default()
        };
        let mut previous = None;
        for child in self.boxes[slot].children.clone() {
            let leaf = self.leaf_of(child, c, lane)?;
            previous = Some(lane.put(child, previous, leaf));
        }
        Ok(())
    }

    /// The children put in since the last layout, each after its
    /// previous sibling's leaf.
    fn fill_inserted(&mut self, c: &Container, lane: &mut Lane) -> Result<(), LaneDecline> {
        for (child, after) in std::mem::take(&mut lane.inserted) {
            let leaf = self.leaf_of(child, c, lane)?;
            let previous = after.map(|after| lane.handle(after));
            lane.put(child, previous, leaf);
        }
        Ok(())
    }

    /// The children built or cleared this update laid out again, their
    /// leaves set: found from the lane's children or from the stale
    /// slots, whichever are fewer, so a lane of a few children under
    /// many restyled rows, or a long list with a few, costs the fewer.
    fn fill_stale(&mut self, c: &Container, lane: &mut Lane) -> Result<(), LaneDecline> {
        let mut stale: Vec<usize> = if lane.handles.len() < self.stale.len() {
            (lane.handles.keys().copied())
                .filter(|slot| self.stale.contains(slot))
                .collect()
        } else {
            (self.stale.iter().copied())
                .filter(|slot| lane.handles.contains_key(slot))
                .collect()
        };
        stale.sort_unstable();
        self.lay_out_again(stale, c, lane)
    }

    /// The `children` of the lane laid out again, their leaves set.
    fn lay_out_again(
        &mut self,
        children: Vec<usize>,
        c: &Container,
        lane: &mut Lane,
    ) -> Result<(), LaneDecline> {
        for child in children {
            let leaf = self.leaf_of(child, c, lane)?;
            lane.tree.set(lane.handle(child), leaf);
            lane.moved.push(child);
        }
        Ok(())
    }

    /// The child in `slot` laid out: its leaf, with its layout held
    /// for `write_starts`.
    fn leaf_of(
        &mut self,
        slot: usize,
        c: &Container,
        lane: &mut Lane,
    ) -> Result<BlockSummary, LaneDecline> {
        let laid = self.lane_child(NodeId::from(slot), c)?;
        lane.pending.insert(slot, laid.layout);
        if reads_heights(&self.boxes[slot].style) {
            lane.height_readers.insert(slot);
        } else {
            lane.height_readers.remove(&slot);
        }
        BlockSummary::leaf(laid.leaf)
    }
}

//! How a lane's leaves are made (`lane_layout.rs` routes a
//! container here): every child into a fresh tree, or only the children
//! whose leaves changed, each laid out by its kind (`lane_kind.rs`) with
//! its layout held for the lane's write.

use rustc_hash::{FxHashMap, FxHashSet};

use super::lane::Lane;
use super::lane_kind::LaneKind;
use super::lane_tree::Handle;
use super::tree::BoxTree;
use crate::lane_decline::LaneDecline;

impl BoxTree<'_> {
    /// The children whose leaves changed laid out again: those built or
    /// cleared and those reading the height bases when they changed,
    /// each once, then those put in, once (a child put in is stale too,
    /// but not yet in the tree when the others are found).
    pub(super) fn fill_changed<K: LaneKind>(
        &mut self,
        c: &K::Container,
        lane: &mut Lane<K>,
        heights_changed: bool,
    ) -> Result<(), LaneDecline> {
        let readers = heights_changed.then_some(&lane.height_readers);
        let again = again(&self.stale, &lane.handles, readers);
        self.lay_out_again(again, c, lane)?;
        self.fill_inserted(c, lane)
    }

    /// Every child laid out and put in a fresh tree, in order.
    pub(super) fn fill_all<K: LaneKind>(
        &mut self,
        slot: usize,
        c: &K::Container,
        lane: &mut Lane<K>,
    ) -> Result<(), LaneDecline> {
        *lane = Lane {
            share: lane.share.take(),
            heights: lane.heights.take(),
            all: false,
            ..Lane::new(K::gap(c))
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
    fn fill_inserted<K: LaneKind>(
        &mut self,
        c: &K::Container,
        lane: &mut Lane<K>,
    ) -> Result<(), LaneDecline> {
        for (child, after) in std::mem::take(&mut lane.inserted) {
            let leaf = self.leaf_of(child, c, lane)?;
            let previous = after.map(|after| lane.handle(after));
            lane.put(child, previous, leaf);
        }
        Ok(())
    }

    /// The `children` of the lane laid out again, their leaves set.
    fn lay_out_again<K: LaneKind>(
        &mut self,
        children: Vec<usize>,
        c: &K::Container,
        lane: &mut Lane<K>,
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
    fn leaf_of<K: LaneKind>(
        &mut self,
        slot: usize,
        c: &K::Container,
        lane: &mut Lane<K>,
    ) -> Result<K::Summary, LaneDecline> {
        let (leaf, layout) = K::lay_out_child(self, slot, c)?;
        lane.pending.insert(slot, layout);
        if K::reads_heights(&self.boxes[slot].style) {
            lane.height_readers.insert(slot);
        } else {
            lane.height_readers.remove(&slot);
        }
        Ok(leaf)
    }
}

/// The lane's children to lay out again, each once and in slot order:
/// those among the `stale` slots, found from the lane's children or
/// from the stale slots, whichever are fewer, so a lane of a few
/// children under many restyled rows, or a long list with a few, costs
/// the fewer; and the height `readers` when the bases changed.
fn again(
    stale: &FxHashSet<usize>,
    handles: &FxHashMap<usize, Handle>,
    readers: Option<&FxHashSet<usize>>,
) -> Vec<usize> {
    let mut again: Vec<usize> = if handles.len() < stale.len() {
        handles
            .keys()
            .copied()
            .filter(|slot| stale.contains(slot))
            .collect()
    } else {
        stale
            .iter()
            .copied()
            .filter(|slot| handles.contains_key(slot))
            .collect()
    };
    again.extend(readers.into_iter().flatten().copied());
    again.sort_unstable();
    again.dedup();
    again
}

#[cfg(test)]
#[path = "lane_fill_tests.rs"]
mod tests;

//! Where each box's clip comes from, by slot (IU3 3c-3c): the content
//! of its containing block, the box it is placed against when it is
//! out of flow, else its parent (`chain.rs`). Kept with paint order and
//! made again only when the tree's shape changes, so redoing the clips
//! under a changed slot (`kept_clips.rs`) finds the boxes whose clip
//! comes from it without a walk over the tree.

use crate::layout::{LayoutBox, containing_blocks};

/// Each slot's clip parent and depth, and the slots each passes its
/// clip to.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct ClipTree {
    /// The box whose content clip a box takes; `None` for the root and
    /// vacant slots.
    parent: Vec<Option<usize>>,
    /// The number of boxes above a box in the tree: a clip parent is
    /// above, so it comes first in depth order.
    depth: Vec<usize>,
    /// Each slot's dependents, in slot order: kept by a splice in place,
    /// a slot put in or taken out of its parent's group, not made again
    /// over every slot.
    dependents: Vec<Vec<usize>>,
}

impl ClipTree {
    /// The clip tree of `boxes`, box 0 the root.
    pub(crate) fn of(boxes: &[LayoutBox]) -> Self {
        let (parent, depth) = parents_and_depths(boxes);
        let dependents = grouped(&parent);
        Self {
            parent,
            depth,
            dependents,
        }
    }

    /// `self` for `len` slots with each of `added` (slot, parent),
    /// parents first, put below its parent and each of `freed` taken
    /// out: an order splice (`order_splice.rs`), where every added box
    /// is in flow, so its clip parent is its parent.
    pub(crate) fn spliced(
        &mut self,
        len: usize,
        added: &[(usize, usize)],
        freed: impl Iterator<Item = usize>,
    ) {
        self.parent.resize(len, None);
        self.depth.resize(len, 0);
        self.dependents.resize_with(len, Vec::new);
        for &(slot, parent) in added {
            self.leave(slot);
            self.parent[slot] = Some(parent);
            self.depth[slot] = self.depth[parent] + 1;
            let group = &mut self.dependents[parent];
            let at = group.partition_point(|&dependent| dependent < slot);
            group.insert(at, slot);
        }
        // every freed slot out of its group before any group is cleared:
        // a freed slot's parent can be freed too
        let freed: Vec<usize> = freed.collect();
        for &slot in &freed {
            self.leave(slot);
            self.depth[slot] = 0;
        }
        for &slot in &freed {
            self.dependents[slot].clear();
        }
    }

    /// Takes `slot` out of its parent's group, if it has a parent;
    /// a panic naming both when the group does not hold it.
    fn leave(&mut self, slot: usize) {
        let Some(parent) = self.parent[slot].take() else {
            return;
        };
        let group = &mut self.dependents[parent];
        let at = group.binary_search(&slot).unwrap_or_else(|_| {
            panic!(
                "slot {slot} has clip parent {parent} but is not among its dependents: the \
                 groups are out of slot order or were not kept by a splice (clip_tree.rs \
                 `spliced`); reproduce with `cargo xtask test`"
            )
        });
        group.remove(at);
    }

    /// The slots in the tree: one past the last.
    pub(crate) fn len(&self) -> usize {
        self.parent.len()
    }

    /// The box whose content clip box `slot` takes.
    pub(crate) fn parent(&self, slot: usize) -> Option<usize> {
        self.parent[slot]
    }

    pub(crate) fn depth(&self, slot: usize) -> usize {
        self.depth[slot]
    }

    /// The boxes whose clip comes from box `slot`'s content.
    pub(crate) fn dependents(&self, slot: usize) -> &[usize] {
        &self.dependents[slot]
    }

    /// The first slot whose parent, depth or dependents differ from
    /// `other`'s; `None` when the trees are equal.
    pub(crate) fn first_differing(&self, other: &Self) -> Option<usize> {
        let slots = self.len().max(other.len());
        (0..slots).find(|&slot| self.at(slot) != other.at(slot))
    }

    fn at(&self, slot: usize) -> Option<(Option<usize>, usize, &[usize])> {
        (slot < self.len()).then(|| (self.parent(slot), self.depth(slot), self.dependents(slot)))
    }
}

/// Each box's clip parent, the containing block of an out-of-flow box
/// else its parent, and its depth in the tree.
fn parents_and_depths(boxes: &[LayoutBox]) -> (Vec<Option<usize>>, Vec<usize>) {
    let containing = containing_blocks(boxes);
    let mut parent = vec![None; boxes.len()];
    let mut depth = vec![0; boxes.len()];
    let mut open: Vec<usize> = (!boxes.is_empty()).then_some(0).into_iter().collect();
    while let Some(index) = open.pop() {
        for &child in &boxes[index].children {
            parent[child] = containing[child].or(Some(index));
            depth[child] = depth[index] + 1;
            open.push(child);
        }
    }
    (parent, depth)
}

/// Each slot's dependents, in slot order.
fn grouped(parent: &[Option<usize>]) -> Vec<Vec<usize>> {
    let mut dependents = vec![Vec::new(); parent.len()];
    for (slot, above) in parent.iter().enumerate() {
        if let Some(above) = *above {
            dependents[above].push(slot);
        }
    }
    dependents
}

#[cfg(test)]
#[path = "clip_tree_tests.rs"]
mod tests;

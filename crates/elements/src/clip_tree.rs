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
    /// Slot `s`'s dependents are `dependents[first[s]..first[s + 1]]`.
    first: Vec<usize>,
    dependents: Vec<usize>,
}

impl ClipTree {
    /// The clip tree of `boxes`, box 0 the root.
    pub(crate) fn of(boxes: &[LayoutBox]) -> Self {
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
        let (first, dependents) = grouped(&parent);
        Self {
            parent,
            depth,
            first,
            dependents,
        }
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
        &self.dependents[self.first[slot]..self.first[slot + 1]]
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

/// Each slot's dependents in one vector, grouped by parent, and where
/// each group starts.
fn grouped(parent: &[Option<usize>]) -> (Vec<usize>, Vec<usize>) {
    let mut first = vec![0; parent.len() + 1];
    for &above in parent.iter().flatten() {
        first[above + 1] += 1;
    }
    for slot in 0..parent.len() {
        first[slot + 1] += first[slot];
    }
    let mut next = first.clone();
    let mut dependents = vec![0; first[parent.len()]];
    for (slot, above) in parent.iter().enumerate() {
        if let Some(above) = *above {
            dependents[next[above]] = slot;
            next[above] += 1;
        }
    }
    (first, dependents)
}

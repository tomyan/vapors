//! A lane child's handle: an index into the tree's slots and the slot's
//! generation, bumped when the child is removed, so a handle kept past
//! its child's removal is refused rather than naming the child that
//! took the index since (review of #293, M2).

use super::{Handle, LaneTree, Summary};

/// One handle index: the leaf holding its child, none once removed,
/// and the generation the index is on.
#[derive(Debug, Default)]
pub(super) struct Slot {
    pub(super) leaf: Option<usize>,
    generation: u32,
}

impl<S: Summary> LaneTree<S> {
    /// The leaf holding `handle`'s child and its index there.
    ///
    /// # Panics
    /// `handle`'s child was removed, or `handle` is another lane's.
    pub(super) fn locate(&self, handle: Handle) -> (usize, usize) {
        let slot = self.leaf_of.get(handle.index as usize);
        let current = slot.filter(|slot| slot.generation == handle.generation);
        let leaf = current.and_then(|slot| slot.leaf).unwrap_or_else(|| {
            panic!(
                "lane handle {handle:?} is not in the tree: its child was removed (the index \
                 may name a child put in since), or it is another lane's; drop a handle when \
                 its child is removed"
            )
        });
        let at = self.nodes[leaf]
            .children
            .iter()
            .position(|&h| h == handle.index);
        (leaf, at.expect("a leaf holds the handles that name it"))
    }

    /// The handle on `index` now: a leaf's child entry as a handle.
    pub(super) fn handle(&self, index: u32) -> Handle {
        Handle {
            index,
            generation: self.leaf_of[index as usize].generation,
        }
    }

    /// A handle for a child put in `leaf`: a freed index on its next
    /// generation, or a new one.
    pub(super) fn new_handle(&mut self, leaf: usize) -> Handle {
        let index = self.free_handles.pop().unwrap_or_else(|| {
            let next = u32::try_from(self.leaf_of.len()).expect("a lane under 2^32 children");
            self.leaf_of.push(Slot::default());
            next
        });
        let slot = &mut self.leaf_of[index as usize];
        slot.leaf = Some(leaf);
        Handle {
            index,
            generation: slot.generation,
        }
    }

    /// Retires `handle`: its index goes to the next generation and is
    /// free for the next insert.
    pub(super) fn forget(&mut self, handle: Handle) {
        let slot = &mut self.leaf_of[handle.index as usize];
        slot.leaf = None;
        slot.generation = slot
            .generation
            .checked_add(1)
            .expect("a lane slot under 2^32 removes");
        self.free_handles.push(handle.index);
    }
}

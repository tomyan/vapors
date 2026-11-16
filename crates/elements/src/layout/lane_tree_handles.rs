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

/// What each handle names, by the handle's index: an index read, where a
/// map by handle hashes (a lane's child slots, read once a child in a
/// pass along the lane). An entry is the handle's only while its
/// generation is the handle's.
#[derive(Debug, Default)]
pub(crate) struct HandleSlots(Vec<Option<(u32, usize)>>);

impl HandleSlots {
    /// The slot `handle` names, if it is held.
    pub(crate) fn get(&self, handle: Handle) -> Option<usize> {
        let held = self.0.get(handle.index as usize).copied().flatten();
        held.filter(|&(generation, _)| generation == handle.generation)
            .map(|(_, slot)| slot)
    }

    /// Notes that `handle` names `slot`.
    pub(crate) fn insert(&mut self, handle: Handle, slot: usize) {
        let at = handle.index as usize;
        if self.0.len() <= at {
            self.0.resize(at + 1, None);
        }
        self.0[at] = Some((handle.generation, slot));
    }

    /// The slot the handle at `index` names, for a child the tree holds
    /// now (`LaneTree::each_start_from`), or a panic naming the index.
    pub(crate) fn at_index(&self, index: u32) -> usize {
        let held = self.0.get(index as usize).copied().flatten();
        held.unwrap_or_else(|| {
            panic!(
                "lane handle index {index} names no slot: a child in the tree is missing from \
                 the lane's slots (lane.rs); reproduce with `cargo xtask test`"
            )
        })
        .1
    }

    /// Forgets `handle`.
    pub(crate) fn remove(&mut self, handle: Handle) {
        if self.get(handle).is_some() {
            self.0[handle.index as usize] = None;
        }
    }
}

impl std::ops::Index<&Handle> for HandleSlots {
    type Output = usize;

    /// The slot `handle` names, or a panic naming the handle: a lane
    /// holds a slot for every handle in its tree.
    fn index(&self, handle: &Handle) -> &usize {
        let held = self.0.get(handle.index as usize).and_then(Option::as_ref);
        let held = held.filter(|(generation, _)| *generation == handle.generation);
        &held
            .unwrap_or_else(|| {
                panic!(
                    "lane handle {handle:?} names no slot: its child was removed from the lane's \
                     slots but not its tree (lane.rs); reproduce with `cargo xtask test`"
                )
            })
            .1
    }
}

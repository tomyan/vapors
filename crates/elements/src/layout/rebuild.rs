//! Rebuild where marked (IU3 PR 1b): an update builds again only the
//! subtrees of the marked elements' owners (`Marks::owners`), in their
//! slots, and keeps every other box with its cache and layout. A
//! rebuilt subtree that changed clears its ancestors' caches and
//! hoisted lists up to the root, as Taffy's `mark_dirty` does, so they
//! are laid out again and hoist their out-of-flow boxes afresh.

use vapors_dom::NodeId;

use super::boxes::{Source, build};
use super::keep::{KeptBoxes, Lender};
use super::marks::Rebuild;
use super::rebuilt::Rebuilt;
use super::slots::{Key, Placed, Taken};
use crate::arena::Arena;
use crate::text_marks::TextMarks;

impl KeptBoxes {
    /// This update's boxes and the slots built and freed: the kept
    /// boxes with each owner's subtree built again, or every box built
    /// when everything is marked, a mark reaches the root with no
    /// owner, nothing is kept or the scale changed. The boxes are no
    /// longer kept until `keep` hands them back, so an update that
    /// fails builds every box next time.
    pub(crate) fn rebuild(
        &mut self,
        source: &Source<'_>,
        root: NodeId,
        scale: f32,
        may_keep: &dyn Fn(NodeId) -> bool,
    ) -> (Placed, Rebuilt) {
        self.mark_text_marks(source.text_marks.as_ref());
        let marks = std::mem::take(&mut self.marks);
        let every =
            marks.all || self.placed.boxes.is_empty() || self.scale.to_bits() != scale.to_bits();
        let rebuild = if every {
            Rebuild::Every
        } else {
            marks.rebuild(source.arena, source.used, &self.placed.slots, root)
        };
        let Rebuild::Owners(owners) = rebuild else {
            return self.build_every(source, root, scale, may_keep);
        };
        let mut kept = std::mem::take(&mut self.placed);
        let built = rebuild_owners(&mut kept, &owners, source, may_keep);
        (kept, built)
    }

    /// Every box built, lent the kept boxes' caches, every slot built.
    fn build_every(
        &mut self,
        source: &Source<'_>,
        root: NodeId,
        scale: f32,
        may_keep: &dyn Fn(NodeId) -> bool,
    ) -> (Placed, Rebuilt) {
        let mut placed = self.place(build(source, root));
        self.lend(&mut placed, scale, may_keep);
        self.placed = Placed::default();
        let rebuilt = Rebuilt::every(&placed.boxes);
        (placed, rebuilt)
    }

    /// Marks the field whose caret or selection the kept boxes show,
    /// and the one focused now, when the two differ.
    fn mark_text_marks(&mut self, now: Option<&(NodeId, TextMarks)>) {
        if self.text_marks.as_ref() == now {
            return;
        }
        for (node, _) in self.text_marks.iter().chain(now) {
            self.marks.mark(*node);
        }
        self.text_marks = now.cloned();
    }
}

/// Builds each owner's subtree again in `placed`; which slots.
fn rebuild_owners(
    placed: &mut Placed,
    owners: &[NodeId],
    source: &Source<'_>,
    may_keep: &dyn Fn(NodeId) -> bool,
) -> Rebuilt {
    let slots: Vec<usize> = owners.iter().map(|&owner| slot_of(placed, owner)).collect();
    let mut taken = take_subtrees(placed, &slots);
    let put = owners
        .iter()
        .flat_map(|&owner| put_owner(placed, source, owner, &mut taken))
        .collect();
    placed.free_taken(&taken);
    let rebuilt = Rebuilt::subtrees(&placed.boxes, put, &taken);
    for (&owner, &slot) in owners.iter().zip(&slots) {
        if !lend_subtree(placed, slot, &taken, may_keep) {
            clear_ancestors(placed, ancestors(source.arena, owner));
        }
    }
    rebuilt
}

/// Takes the subtrees at `slots` out of `placed`.
fn take_subtrees(placed: &mut Placed, slots: &[usize]) -> Taken {
    let mut taken = Taken::default();
    for &slot in slots {
        placed.take_subtree(slot, &mut taken);
    }
    taken
}

/// Builds `owner`'s subtree into `placed`; the slots its boxes took.
fn put_owner(
    placed: &mut Placed,
    source: &Source<'_>,
    owner: NodeId,
    taken: &mut Taken,
) -> Vec<usize> {
    placed.put_subtree(build(source, owner), taken)
}

fn slot_of(placed: &Placed, owner: NodeId) -> usize {
    placed
        .slots
        .of(Key::Element(owner))
        .expect("an owner holds a slot")
}

/// Lends the taken boxes' caches to the rebuilt subtree at `slot`;
/// whether the whole subtree was unchanged.
fn lend_subtree(
    placed: &mut Placed,
    slot: usize,
    taken: &Taken,
    may_keep: &dyn Fn(NodeId) -> bool,
) -> bool {
    let before = |at: usize| taken.boxes.get(&at).map(|(key, old)| (*key, old));
    let mut lender = Lender {
        before: &before,
        new: &placed.slots,
        may_keep,
        lent: 0,
    };
    lender.lend_from(&mut placed.boxes, slot)
}

/// Clears the cache and hoisted list of each of `ancestors` that
/// holds a box. Only element boxes have children, so these are every
/// box above a rebuilt subtree.
fn clear_ancestors(placed: &mut Placed, ancestors: impl Iterator<Item = NodeId>) {
    for ancestor in ancestors {
        if let Some(slot) = placed.slots.of(Key::Element(ancestor)) {
            placed.boxes[slot].cache.clear();
            placed.boxes[slot].hoisted.clear();
        }
    }
}

/// The elements above `node`, nearest first.
fn ancestors(arena: &Arena, node: NodeId) -> impl Iterator<Item = NodeId> + '_ {
    let parent = |&node: &NodeId| arena.get(node, "clear_ancestors").parent;
    std::iter::successors(parent(&node), parent)
}

#[cfg(test)]
#[path = "rebuild_tests.rs"]
mod tests;

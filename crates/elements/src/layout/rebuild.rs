//! Rebuild where marked (IU3 PR 1b): an update builds again only the
//! subtrees of the marked elements' owners (`Marks::owners`), in their
//! slots, and keeps every other box with its cache and layout. A
//! rebuilt subtree that changed clears its ancestors' caches and
//! hoisted lists up to the root, as Taffy's `mark_dirty` does, so they
//! are laid out again and hoist their out-of-flow boxes afresh.

use vapors_dom::NodeId;

use super::boxes::{Source, build};
use super::keep::{KeptBoxes, Lender};
use super::slots::{Key, Placed, Taken};
use crate::text_marks::TextMarks;

impl KeptBoxes {
    /// This update's boxes and how many were built: the kept boxes with
    /// each owner's subtree built again, or every box built when
    /// everything is marked, nothing is kept or the scale changed. The
    /// boxes are no longer kept until `keep` hands them back, so an
    /// update that fails builds every box next time.
    pub(crate) fn rebuild(
        &mut self,
        source: &Source<'_>,
        root: NodeId,
        scale: f32,
        may_keep: &dyn Fn(NodeId) -> bool,
    ) -> (Placed, usize) {
        self.mark_text_marks(source.text_marks.as_ref());
        let marks = std::mem::take(&mut self.marks);
        if marks.all || self.placed.boxes.is_empty() || self.scale.to_bits() != scale.to_bits() {
            let built = build(source, root);
            let count = built.len();
            let mut placed = self.place(built);
            self.lend(&mut placed, scale, may_keep);
            self.placed = Placed::default();
            return (placed, count);
        }
        let mut kept = std::mem::take(&mut self.placed);
        let owners = marks.owners(source.arena, source.used, &kept.slots, root);
        let count = rebuild_owners(&mut kept, &owners, source, may_keep);
        (kept, count)
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

/// Builds each owner's subtree again in `placed`; how many boxes.
fn rebuild_owners(
    placed: &mut Placed,
    owners: &[NodeId],
    source: &Source<'_>,
    may_keep: &dyn Fn(NodeId) -> bool,
) -> usize {
    let mut taken = Taken::default();
    let slots: Vec<usize> = owners
        .iter()
        .map(|&owner| {
            placed
                .slots
                .of(Key::Element(owner))
                .expect("an owner holds a slot")
        })
        .collect();
    for &slot in &slots {
        placed.take_subtree(slot, &mut taken);
    }
    let mut count = 0;
    for &owner in owners {
        let built = build(source, owner);
        count += built.len();
        placed.put_subtree(built, &mut taken);
    }
    placed.free_taken(&taken);
    let parents = parents(placed);
    for &slot in &slots {
        if !lend_subtree(placed, slot, &taken, may_keep) {
            clear_ancestors(placed, &parents, slot);
        }
    }
    count
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

/// Clears the cache and hoisted list of each ancestor of `slot`.
fn clear_ancestors(placed: &mut Placed, parents: &[Option<usize>], slot: usize) {
    let mut at = parents[slot];
    while let Some(ancestor) = at {
        placed.boxes[ancestor].cache.clear();
        placed.boxes[ancestor].hoisted.clear();
        at = parents[ancestor];
    }
}

/// Each slot's parent slot; none for the root and vacant slots.
fn parents(placed: &Placed) -> Vec<Option<usize>> {
    let mut parents = vec![None; placed.boxes.len()];
    for (slot, layout_box) in placed.boxes.iter().enumerate() {
        for &child in &layout_box.children {
            parents[child] = Some(slot);
        }
    }
    parents
}

#[cfg(test)]
#[path = "rebuild_tests.rs"]
mod tests;

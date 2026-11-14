//! Rebuild where marked (IU3 PR 1b): an update builds again only the
//! subtrees of the marked elements' owners (`Marks::owners`), in their
//! slots, and keeps every other box with its cache and layout. A
//! rebuilt subtree that changed is handed to layout as a changed owner
//! (`isolated.rs`), with the slots of its ancestors up to the root and
//! what the box it replaced held, to be laid out in isolation or, as
//! Taffy's `mark_dirty` does, to have those ancestors laid out again.

use vapors_dom::NodeId;

use rustc_hash::FxHashMap;

use super::boxes::{Source, build, build_shallow};
use super::keep::{KeptBoxes, Lender};
use super::marks::{Build, Owner, Rebuild};
use super::rebuilt::Rebuilt;
use super::slots::{Key, Placed, Taken};
use crate::text_marks::TextMarks;
use changed::changed_owners;

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
            marks.rebuild(source.arena, source.used, &self.placed, root)
        };
        let Rebuild::Owners(owners, unowned) = rebuild else {
            return self.build_every(source, root, scale, may_keep);
        };
        let mut kept = std::mem::take(&mut self.placed);
        let mut built = rebuild_owners(&mut kept, &owners, source, may_keep);
        built.edits.count_unowned(&unowned);
        // Debug timings include this full box build on every narrowed
        // update; the test that would retire it is the one
        // `layout_check.rs` names.
        #[cfg(debug_assertions)]
        super::rebuild_check::check_against_fresh(&kept, source, root, &built.slots);
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

/// Builds each owner's boxes again in `placed`; which slots.
fn rebuild_owners(
    placed: &mut Placed,
    owners: &[Owner],
    source: &Source<'_>,
    may_keep: &dyn Fn(NodeId) -> bool,
) -> Rebuilt {
    let slots: Vec<usize> = owners
        .iter()
        .map(|owner| slot_of(placed, owner.node))
        .collect();
    let mut taken = take_owners(placed, owners, &slots);
    let put = owners
        .iter()
        .flat_map(|owner| put_owner(placed, source, owner, &mut taken))
        .collect();
    placed.free_taken(&taken);
    let mut rebuilt = Rebuilt::subtrees(&placed.boxes, put, &taken);
    rebuilt.edits = super::rebuilt::EditCounts::of(owners.iter().map(|owner| &owner.build));
    let nodes: Vec<NodeId> = owners.iter().map(|owner| owner.node).collect();
    let changed = lend_owners(placed, (&nodes, &slots), &taken, may_keep);
    rebuilt.changed = changed_owners(placed, source.arena, &changed, (taken, may_keep));
    rebuilt
}

/// Lends the taken boxes' caches to each owner's rebuilt subtree, at
/// its slot; the owners whose subtree changed, with their slots. Every
/// owner is lent before any is settled: an owner can sit in a shallow
/// owner's kept child.
fn lend_owners(
    placed: &mut Placed,
    (owners, slots): (&[NodeId], &[usize]),
    taken: &Taken,
    may_keep: &dyn Fn(NodeId) -> bool,
) -> Vec<(NodeId, usize)> {
    (owners.iter().zip(slots))
        .filter(|&(_, &slot)| !lend_subtree(placed, slot, taken, may_keep))
        .map(|(&owner, &slot)| (owner, slot))
        .collect()
}

/// Takes each owner's boxes, at `slots`, out of `placed`: a shallow
/// owner's own box and the subtrees of the children it does not keep.
fn take_owners(placed: &mut Placed, owners: &[Owner], slots: &[usize]) -> Taken {
    let mut taken = Taken::default();
    for (owner, &slot) in owners.iter().zip(slots) {
        match &owner.build {
            Build::Shallow(kept, _) => placed.take_shallow(slot, kept, &mut taken),
            Build::Edited(edits) => placed.take_edited(slot, edits, &mut taken),
            Build::Full => placed.take_subtree(slot, &mut taken),
        }
    }
    taken
}

/// Builds `owner`'s boxes into `placed`; the slots the built boxes took.
fn put_owner(
    placed: &mut Placed,
    source: &Source<'_>,
    owner: &Owner,
    taken: &mut Taken,
) -> Vec<usize> {
    match &owner.build {
        Build::Shallow(kept, _) => {
            let (built, placeholders) = build_shallow(source, owner.node, kept);
            placed.put_subtree(built, &placeholders, taken)
        }
        Build::Edited(edits) => placed.put_edited(source, (owner.node, edits), taken),
        Build::Full => placed.put_subtree(build(source, owner.node), &FxHashMap::default(), taken),
    }
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

#[path = "rebuild_changed.rs"]
mod changed;

#[cfg(test)]
#[path = "rebuild_tests.rs"]
mod tests;

//! An owner's box children edited in place (`child_edits.rs`): its box
//! taken and put back with the removed children's slots taken out and
//! each inserted child's subtree built and put in after its previous
//! sibling's box. The other children are neither walked nor taken.

use rustc_hash::FxHashMap;
use taffy::Display;
use vapors_dom::NodeId;

use super::super::boxes::{Source, build, element_box};
use super::super::child_edits::Edits;
use super::{Key, Placed, Taken};

impl Placed {
    /// Takes out the box at `slot`, `owner`'s, and the subtrees of the
    /// children `edits` removes.
    pub(crate) fn take_edited(&mut self, slot: usize, edits: &Edits, taken: &mut Taken) {
        self.take_box(slot, taken);
        for &removed in &edits.removed {
            self.take_subtree(removed, taken);
        }
    }

    /// Puts `owner`'s box back with its children edited; the slots of
    /// the boxes built, the owner's first, and those of the inserted
    /// children's boxes, the roots of the subtrees built.
    ///
    /// # Panics
    /// The owner's box was not taken, or a removed slot or an inserted
    /// child's previous sibling is not among its children: `edits` was
    /// not checked against these boxes (`Resolver::edits`).
    pub(crate) fn put_edited(
        &mut self,
        source: &Source<'_>,
        (owner, edits): (NodeId, &Edits),
        taken: &mut Taken,
    ) -> (Vec<usize>, Vec<usize>) {
        let key = Key::Element(owner);
        let slot = self.slot_for(key, taken);
        let boxed = !edits.removed.is_empty() || !edits.inserted.is_empty();
        let mut children = old_children(taken, slot, boxed);
        if boxed {
            let reshaped = self.reshaped_by(source, (slot, &children, edits), taken);
            taken.reshaped.insert(slot, reshaped);
        }
        for &removed in &edits.removed {
            let at = self.child_position(slot, &children, removed);
            children.remove(at);
            self.lanes
                .borrow_mut()
                .child_removed(slot, removed, children.get(at).copied());
        }
        let mut put = vec![slot];
        let mut roots = Vec::new();
        let mut pending = edits.inserted.clone();
        while let Some(child) = next_in_order(source, &mut pending) {
            let at = self.after_previous(source, (slot, child, &children), taken);
            let built = self.put_subtree(build(source, child), &FxHashMap::default(), taken);
            let after = at.checked_sub(1).map(|before| children[before]);
            children.insert(at, built[0]);
            roots.push(built[0]);
            self.lanes
                .borrow_mut()
                .child_inserted(slot, built[0], after);
            put.extend(built);
        }
        let (mut own, field_text) = element_box(source, owner);
        assert!(
            field_text.is_none(),
            "an edited owner {owner:?} is a text field"
        );
        own.children = children;
        self.hold(key, slot);
        self.boxes[slot] = own;
        (put, roots)
    }

    /// The slot `node`'s box holds, or held when another owner rebuilt
    /// this update took it and has not put it back yet.
    pub(super) fn slot_held(&self, node: NodeId, taken: &Taken) -> usize {
        let key = Key::Element(node);
        let slot = self.slots.of(key).or_else(|| taken.held(key));
        slot.unwrap_or_else(|| panic!("{node:?}, before an inserted child, has no box"))
    }

    /// Where `child` goes among `children`, the owner's in `owner`: after
    /// the box of its nearest previous sibling that makes one, or first.
    fn after_previous(
        &self,
        source: &Source<'_>,
        (owner, child, children): (usize, NodeId, &[usize]),
        taken: &Taken,
    ) -> usize {
        match previous_box(source, child) {
            Some(node) => self.child_position(owner, children, self.slot_held(node, taken)) + 1,
            None => 0,
        }
    }
}

/// Takes out of `pending` an inserted child whose previous sibling with
/// a box is not pending too, so each is put in after a box already put:
/// in DOM order, whatever order they were inserted in (review of #285,
/// H2). None when nothing is pending.
fn next_in_order(source: &Source<'_>, pending: &mut Vec<NodeId>) -> Option<NodeId> {
    let waits = |child: &NodeId| previous_box(source, *child).is_some_and(|p| pending.contains(&p));
    let ready = pending.iter().position(|child| !waits(child));
    if pending.is_empty() {
        return None;
    }
    let ready =
        ready.expect("inserted children form a chain of previous siblings, so one is first");
    Some(pending.remove(ready))
}

/// `child`'s nearest previous sibling that makes a box, past empty text
/// and `display: none`: `Resolver::edits` let no other text or inline
/// element through.
pub(super) fn previous_box(source: &Source<'_>, child: NodeId) -> Option<NodeId> {
    let previous = |node: NodeId| source.arena.get(node, "put_edited").previous;
    let mut at = previous(child);
    while let Some(node) = at {
        let shown = source
            .used
            .get(&node)
            .is_some_and(|used| used.layout.display != Display::None);
        if shown {
            return Some(node);
        }
        at = previous(node);
    }
    None
}

/// The edited owner's old children, from its box taken at `slot`: moved
/// out when its edits insert or remove a box (debug builds keep a copy,
/// `Taken::old_children`), copied when they change nothing, as lending
/// then compares the old box's children (`Lender::lend_from`). Moved
/// out, the old box's children read empty: lending finds them changed,
/// as the edits changed them, and `reshaped` refuses the slot.
fn old_children(taken: &mut Taken, slot: usize, boxed: bool) -> Vec<usize> {
    let (_, old) = taken
        .boxes
        .get_mut(&slot)
        .unwrap_or_else(|| panic!("the edited owner's box in slot {slot} was not taken"));
    if !boxed {
        return old.children.clone();
    }
    let children = std::mem::take(&mut old.children);
    taken.moved_out.insert(slot);
    #[cfg(debug_assertions)]
    taken.old_children.insert(slot, children.clone());
    children
}

/// `slot`'s index in `children`, by a linear search.
pub(super) fn position(children: &[usize], slot: usize) -> usize {
    children
        .iter()
        .position(|&child| child == slot)
        .unwrap_or_else(|| panic!("slot {slot} is not a child of the edited owner's box"))
}

#[path = "slots_edit_places.rs"]
mod places;

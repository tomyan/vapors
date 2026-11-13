//! Inserts and removes under an element, recorded as the document makes
//! them, so a rebuild can edit the element's box children in place
//! rather than walk every child (`docs/plans/splice-layout.md`, section
//! 7, PR 1). An edit is used only where no text run can change: each
//! inserted or removed child is a block-level element between block
//! boxes or the ends. Anything else builds the element shallow, as
//! before (`owners.rs`).

use taffy::Display;
use vapors_dom::NodeId;

use super::keep::KeptBoxes;
use super::owners::Resolver;
use super::slots::Key;
use crate::arena::Arena;
use crate::node::Kind;

/// The inserts and removes under one element since the last update.
#[derive(Debug, Default)]
pub(crate) struct ChildEdits {
    /// Children inserted, in the order they were.
    inserted: Vec<NodeId>,
    /// Children removed: each one's box slot and its neighbours then.
    removed: Vec<Removed>,
    /// A change an edit cannot make: a removed child with no box.
    refused: bool,
}

#[derive(Debug)]
struct Removed {
    slot: usize,
    neighbours: [Option<NodeId>; 2],
}

/// What an element's edit takes out and puts in, checked.
#[derive(Debug, PartialEq)]
pub(crate) struct Edits {
    /// The slots of the removed children's boxes.
    pub(crate) removed: Vec<usize>,
    /// The inserted children that make a box, in the order inserted.
    pub(crate) inserted: Vec<NodeId>,
}

impl KeptBoxes {
    /// Marks `parent`'s children and records `child` inserted under
    /// it, already in place.
    pub(crate) fn note_inserted(&mut self, parent: NodeId, child: NodeId) {
        self.marks.mark_children(parent);
        self.marks
            .edits
            .entry(parent)
            .or_default()
            .inserted
            .push(child);
    }

    /// Marks `parent`'s children and records `child` about to be
    /// removed from it, still in place.
    pub(crate) fn note_removed(&mut self, arena: &Arena, parent: NodeId, child: NodeId) {
        self.marks.mark_children(parent);
        let edits = self.marks.edits.entry(parent).or_default();
        let Some(slot) = self.placed.slots.of(Key::Element(child)) else {
            // a child inserted since the last update has no box to take
            // out; any other child without one could be in a text run
            let before = edits.inserted.len();
            edits.inserted.retain(|&inserted| inserted != child);
            edits.refused |= edits.inserted.len() == before;
            return;
        };
        let node = arena.get(child, "note_removed");
        let neighbours = [node.previous, node.next];
        edits.removed.push(Removed { slot, neighbours });
    }
}

impl Resolver<'_> {
    /// `owner`'s recorded edits, when an edit can make them: every
    /// removed slot a child of `owner`'s box, every inserted child new,
    /// and no text run beside any of them.
    pub(super) fn edits(&self, owner: NodeId, recorded: &ChildEdits) -> Option<Edits> {
        if !self.arena.contains(owner) {
            return None;
        }
        let slot = self.placed.slots.of(Key::Element(owner))?;
        let children = &self.placed.boxes[slot].children;
        if recorded.refused || crate::svg::is(self.arena, owner, "svg") {
            return None;
        }
        let removed = recorded.removed.iter();
        let removed = removed.map(|removed| self.removed(owner, children, removed));
        let removed = removed.collect::<Option<Vec<usize>>>()?;
        let inserted = self.inserted(owner, &recorded.inserted)?;
        Some(Edits { removed, inserted })
    }

    /// `removed`'s slot, when its box is a child of `owner`'s, its
    /// element is no longer `owner`'s child, and it was between block
    /// boxes or the ends.
    fn removed(&self, owner: NodeId, children: &[usize], removed: &Removed) -> Option<usize> {
        let gone = children.contains(&removed.slot) && self.gone_from(removed.slot, owner);
        let [previous, next] = removed.neighbours;
        let beside = self.block_or_end(owner, previous, Step::Previous)
            && self.block_or_end(owner, next, Step::Next);
        (gone && beside).then_some(removed.slot)
    }

    /// The inserted children that make a box; none when one cannot be
    /// put in by an edit.
    fn inserted(&self, owner: NodeId, inserted: &[NodeId]) -> Option<Vec<NodeId>> {
        let mut boxed = Vec::new();
        for &child in inserted {
            if !self.new_child(owner, child) {
                return None;
            }
            if self.used[&child].layout.display == Display::None {
                continue;
            }
            if self.used[&child].inline || !self.beside_blocks(owner, child) {
                return None;
            }
            boxed.push(child);
        }
        Some(boxed)
    }

    /// Whether `child` is an element under `owner`. One moved in from
    /// elsewhere may hold a slot still: its old parent's rebuild takes
    /// it out before any box is put, and `put_subtree` refuses a key
    /// that holds a slot outside every taken subtree.
    fn new_child(&self, owner: NodeId, child: NodeId) -> bool {
        self.arena.contains(child)
            && self.arena.get(child, "edits").parent == Some(owner)
            && self.arena.get(child, "edits").element().is_some()
    }

    /// Whether the element whose box is in `slot` is no longer a child
    /// of `owner`.
    fn gone_from(&self, slot: usize, owner: NodeId) -> bool {
        let Some(Key::Element(node)) = self.placed.slots.key(slot) else {
            return false;
        };
        !self.arena.contains(node) || self.arena.get(node, "edits").parent != Some(owner)
    }

    fn beside_blocks(&self, owner: NodeId, child: NodeId) -> bool {
        let node = self.arena.get(child, "edits");
        self.block_or_end(owner, node.previous, Step::Previous)
            && self.block_or_end(owner, node.next, Step::Next)
    }

    /// Whether the first sibling from `from` on in `step`'s direction
    /// that is not `display: none` is a block-level element of
    /// `owner`, or there is none: no text run to join or split.
    fn block_or_end(&self, owner: NodeId, from: Option<NodeId>, step: Step) -> bool {
        let mut at = from;
        while let Some(node) = at {
            if !self.arena.contains(node) || self.arena.get(node, "edits").parent != Some(owner) {
                return false;
            }
            let Kind::Element(_) = self.arena.get(node, "edits").kind else {
                return false;
            };
            let Some(used) = self.used.get(&node) else {
                return false;
            };
            if used.layout.display != Display::None {
                return !used.inline;
            }
            at = step.from(self.arena, node);
        }
        true
    }
}

#[derive(Clone, Copy)]
enum Step {
    Previous,
    Next,
}

impl Step {
    fn from(self, arena: &Arena, node: NodeId) -> Option<NodeId> {
        let node = arena.get(node, "edits");
        match self {
            Self::Previous => node.previous,
            Self::Next => node.next,
        }
    }
}

//! Inserts and removes under an element, recorded as the document makes
//! them, so a rebuild can edit the element's box children in place
//! rather than build every child again (`docs/plans/splice-layout.md`,
//! section 7, PR 1). The edit costs O(children) in copies and searches
//! of the box's child list, not a build: no child is walked in the
//! DOM. It is made only where no text run can change: each inserted or
//! removed child a block-level element between block boxes or the ends,
//! empty text nodes (a `{for}`'s markers and anchors) passed over, as
//! they make no box. Anything else builds the element shallow, as
//! before, and says why (`EditDecline`, `child_edits_check.rs`).

use vapors_dom::NodeId;

use super::keep::KeptBoxes;
use super::slots::Key;
use crate::arena::Arena;
use crate::edit_decline::EditDecline;
use crate::node::Kind;

/// The inserts and removes under one element since the last update.
#[derive(Debug, Default)]
pub(crate) struct ChildEdits {
    /// Children inserted, in the order they were.
    pub(super) inserted: Vec<NodeId>,
    /// Children removed: each one's box slot and its neighbours then.
    pub(super) removed: Vec<Removed>,
    /// A change an edit cannot make.
    pub(super) refused: Option<EditDecline>,
}

#[derive(Debug)]
pub(super) struct Removed {
    pub(super) slot: usize,
    /// The nearest siblings on each side that are not empty text.
    pub(super) neighbours: [Option<NodeId>; 2],
}

/// What an element's edit takes out and puts in, checked.
#[derive(Debug, PartialEq)]
pub(crate) struct Edits {
    /// The slots of the removed children's boxes.
    pub(crate) removed: Vec<usize>,
    /// The inserted children that make a box, in the order inserted.
    pub(crate) inserted: Vec<NodeId>,
    /// Every inserted element, those with `display: none` too: a change
    /// under one is the edit's own.
    pub(crate) new_children: Vec<NodeId>,
}

impl Edits {
    /// Whether a box change at `node`, under `owner`, is inside a child
    /// the edit builds.
    pub(crate) fn covers(&self, arena: &Arena, node: NodeId, owner: NodeId) -> bool {
        let mut at = node;
        while let Some(parent) = arena.get(at, "covers").parent {
            if parent == owner {
                return self.new_children.contains(&at);
            }
            at = parent;
        }
        false
    }
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
            return no_box_removed(edits, arena, child);
        };
        let node = arena.get(child, "note_removed");
        let neighbours = [
            past_empty_text(arena, node.previous, |n| n.previous),
            past_empty_text(arena, node.next, |n| n.next),
        ];
        edits.removed.push(Removed { slot, neighbours });
    }
}

/// A removed child with no box: one inserted since the last update is
/// dropped from the inserts, an empty text is nothing, and any other
/// (text with characters, inline, hidden) refuses the edit.
fn no_box_removed(edits: &mut ChildEdits, arena: &Arena, child: NodeId) {
    let before = edits.inserted.len();
    edits.inserted.retain(|&inserted| inserted != child);
    if edits.inserted.len() == before && !is_empty_text(arena, child) {
        edits.refused = Some(EditDecline::RemovedWithoutBox);
    }
}

/// The first of `from` and the siblings `step` leads to that is not an
/// empty text node.
pub(super) fn past_empty_text(
    arena: &Arena,
    from: Option<NodeId>,
    step: impl Fn(&crate::node::Node) -> Option<NodeId>,
) -> Option<NodeId> {
    let mut at = from;
    while let Some(node) = at.filter(|&node| is_empty_text(arena, node)) {
        at = step(arena.get(node, "past_empty_text"));
    }
    at
}

/// Whether `node` is a text node with no characters: no box, no run.
pub(super) fn is_empty_text(arena: &Arena, node: NodeId) -> bool {
    arena.contains(node)
        && matches!(&arena.get(node, "edits").kind, Kind::Text(text) if text.is_empty())
}

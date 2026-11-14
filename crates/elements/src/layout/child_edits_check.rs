//! Whether an element's recorded inserts and removes can be made as an
//! edit of its box children (`child_edits.rs`), and if not, why.

use taffy::Display;
use vapors_dom::NodeId;

use super::child_edits::{ChildEdits, Edits, Removed, is_empty_text, past_empty_text};
use super::owners::Resolver;
use super::slots::Key;
use crate::edit_decline::EditDecline;
use crate::node::Kind;

impl Resolver<'_> {
    /// `owner`'s recorded edits, checked against its kept box.
    ///
    /// # Errors
    /// Why an edit cannot make them.
    pub(super) fn edits(&self, owner: NodeId, recorded: &ChildEdits) -> Result<Edits, EditDecline> {
        let in_tree = Some(owner).filter(|&owner| self.arena.contains(owner));
        let slot = in_tree.and_then(|owner| self.slot(owner));
        let slot = slot.ok_or(EditDecline::OwnerHasNoBox)?;
        let kept = &self.placed.boxes[slot];
        if kept.text_field || crate::svg::is(self.arena, owner, "svg") {
            return Err(EditDecline::OwnerNotABlock);
        }
        if let Some(refused) = recorded.refused {
            return Err(refused);
        }
        let removed = recorded.removed.iter();
        let removed = removed.map(|removed| self.removed(owner, &kept.children, removed));
        let removed = removed.collect::<Result<Vec<usize>, _>>()?;
        let inserted = self.inserted(owner, &recorded.inserted)?;
        let new_children = recorded.inserted.clone();
        Ok(Edits {
            removed,
            inserted,
            new_children,
        })
    }

    /// `removed`'s slot, when its box is a child of `owner`'s, its
    /// element is gone from `owner`, and it was between blocks.
    fn removed(
        &self,
        owner: NodeId,
        children: &[usize],
        removed: &Removed,
    ) -> Result<usize, EditDecline> {
        if !children.contains(&removed.slot) || !self.gone_from(removed.slot, owner) {
            return Err(EditDecline::Moved);
        }
        let [previous, next] = removed.neighbours;
        if !(self.block_or_end(owner, previous, Step::Previous)
            && self.block_or_end(owner, next, Step::Next))
        {
            return Err(EditDecline::TextBeside);
        }
        Ok(removed.slot)
    }

    /// The inserted children that make a box.
    fn inserted(&self, owner: NodeId, inserted: &[NodeId]) -> Result<Vec<NodeId>, EditDecline> {
        let mut boxed = Vec::new();
        for &child in inserted {
            if is_empty_text(self.arena, child) {
                continue;
            }
            let used = self
                .used
                .get(&child)
                .filter(|_| self.child_element(owner, child));
            let used = used.ok_or(EditDecline::InsertedInline)?;
            if used.layout.display == Display::None {
                continue;
            }
            if used.inline {
                return Err(EditDecline::InsertedInline);
            }
            let node = self.arena.get(child, "edits");
            let (previous, next) = (node.previous, node.next);
            if !(self.block_or_end(owner, previous, Step::Previous)
                && self.block_or_end(owner, next, Step::Next))
            {
                return Err(EditDecline::TextBeside);
            }
            boxed.push(child);
        }
        Ok(boxed)
    }

    /// Whether `child` is an element under `owner`. One moved in from
    /// elsewhere may hold a slot still: its old parent's rebuild takes
    /// it out before any box is put, and `put_subtree` refuses a key
    /// that holds a slot outside every taken subtree.
    fn child_element(&self, owner: NodeId, child: NodeId) -> bool {
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

    /// Whether the first sibling from `from` on in `step`'s direction
    /// that is neither empty text nor `display: none` is a block-level
    /// element of `owner`, or there is none: no text run to join or
    /// split.
    fn block_or_end(&self, owner: NodeId, from: Option<NodeId>, step: Step) -> bool {
        let mut at = past_empty_text(self.arena, from, |n| step.of(n));
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
            at = past_empty_text(self.arena, step.of(self.arena.get(node, "edits")), |n| {
                step.of(n)
            });
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
    fn of(self, node: &crate::node::Node) -> Option<NodeId> {
        match self {
            Self::Previous => node.previous,
            Self::Next => node.next,
        }
    }
}

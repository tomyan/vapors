//! Where a node new or moved goes (R6): under its parent, before the
//! next node in document order under the same parent that the mirror
//! holds after the take, found by scanning forward from the node's
//! position and into the subtrees of elements that are not nodes. The
//! placements are ordered so that each `before` is placed before the
//! placement naming it.

use vapors_dom::NodeId;

use crate::a11y_mirror::AxPlace;
use crate::a11y_mirror_diff::{After, Diff};
use crate::document::Document;
use crate::node_map::NodeSet;

impl Document {
    /// Placements for what `diff` found to place.
    pub(crate) fn ax_placements(&self, diff: &Diff) -> Vec<AxPlace> {
        let pending = diff
            .to_place
            .iter()
            .map(|&(node, parent)| AxPlace {
                node,
                parent,
                before: self.ax_before(node, parent, diff),
            })
            .collect();
        order(pending)
    }

    /// The next node under `parent` after `node` in document order that
    /// is in the tree after the take.
    fn ax_before(&self, node: NodeId, parent: Option<NodeId>, diff: &Diff) -> Option<NodeId> {
        let mut cursor = node;
        let mut descend = false;
        loop {
            cursor = self.ax_next(cursor, parent, descend)?;
            match diff.after(cursor, &self.ax.records) {
                After::Under(under) if under == parent => return Some(cursor),
                After::Under(_) => descend = false,
                After::Gone => descend = true,
            }
        }
    }

    /// The node after `cursor` in document order, into its children
    /// when `descend`, never past `parent`'s subtree.
    fn ax_next(&self, cursor: NodeId, parent: Option<NodeId>, descend: bool) -> Option<NodeId> {
        if let Some(child) = self.first_child(cursor).filter(|_| descend) {
            return Some(child);
        }
        let mut up = cursor;
        loop {
            if let Some(next) = self.next_sibling(up) {
                return Some(next);
            }
            up = self.parent(up)?;
            if Some(up) == parent {
                return None;
            }
        }
    }

    fn first_child(&self, node: NodeId) -> Option<NodeId> {
        self.arena.get(node, "first child").first_child
    }

    fn next_sibling(&self, node: NodeId) -> Option<NodeId> {
        self.arena.get(node, "next sibling").next
    }
}

/// `pending` in an order a mirror applies one by one: a placement
/// comes after the one placing its `before`.
///
/// # Panics
/// Placements that wait on each other, which document order cannot
/// produce: a bug in vapors.
fn order(mut pending: Vec<AxPlace>) -> Vec<AxPlace> {
    let mut out = Vec::with_capacity(pending.len());
    while !pending.is_empty() {
        let waiting: NodeSet = pending.iter().map(|place| place.node).collect();
        let (ready, rest): (Vec<AxPlace>, Vec<AxPlace>) = pending
            .into_iter()
            .partition(|place| place.before.is_none_or(|before| !waiting.contains(&before)));
        assert!(
            !ready.is_empty(),
            "a11y placements wait on each other ({rest:?}): a bug in vapors; report it with the page"
        );
        out.extend(ready);
        pending = rest;
    }
    out
}

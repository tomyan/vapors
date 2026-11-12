//! Where a node new or moved goes (R6): under its parent, before the
//! next node in document order under the same parent that the mirror
//! holds after the take, found by scanning forward from the node's
//! position and into the subtrees of elements that are not nodes. The
//! placements are ordered so that each `before` is placed before the
//! placement naming it, in one pass over them.

use vapors_dom::NodeId;

use crate::a11y_error::AxError;
use crate::a11y_mirror::AxPlace;
use crate::a11y_mirror_diff::{After, Diff};
use crate::document::Document;
use crate::node_map::{NodeMap, NodeSet};

impl Document {
    /// Placements for what `diff` found to place, ordered, with how
    /// many the ordering considered.
    ///
    /// # Errors
    /// [`AxError::PlacementCycle`]: a bug in vapors.
    pub(crate) fn ax_placements(&self, diff: &Diff) -> Result<(Vec<AxPlace>, usize), AxError> {
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
/// comes after the one placing its `before`. Each placement waits on
/// the one its `before` names, when that is pending too; the rest are
/// ready, and each placed releases the ones waiting on it: O(n), and
/// the steps counted (each placement sorted once, released at most
/// once, placed once) say so.
///
/// # Errors
/// [`AxError::PlacementCycle`]: placements wait on each other, which
/// document order cannot produce: a bug in vapors.
fn order(pending: Vec<AxPlace>) -> Result<(Vec<AxPlace>, usize), AxError> {
    let total = pending.len();
    let mut steps = 0;
    let waiting: NodeSet = pending.iter().map(|place| place.node).collect();
    let mut on: NodeMap<Vec<AxPlace>> = NodeMap::default();
    let mut ready = Vec::new();
    for place in pending {
        steps += 1;
        match place.before.filter(|before| waiting.contains(before)) {
            Some(before) => on.entry(before).or_default().push(place),
            None => ready.push(place),
        }
    }
    let mut out = Vec::with_capacity(total);
    while let Some(place) = ready.pop() {
        let released = on.remove(&place.node).unwrap_or_default();
        steps += 1 + released.len();
        ready.extend(released);
        out.push(place);
    }
    if out.len() == total {
        return Ok((out, steps));
    }
    let mut nodes: Vec<u64> = on
        .into_values()
        .flatten()
        .map(|place| place.node.0)
        .collect();
    nodes.sort_unstable();
    Err(AxError::PlacementCycle { nodes })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(node: u64, before: Option<u64>) -> AxPlace {
        AxPlace {
            node: NodeId(node),
            parent: None,
            before: before.map(NodeId),
        }
    }

    #[test]
    fn a_chain_of_placements_puts_each_before_after_its_own_placement() {
        // Given: 1 before 2, 2 before 3, 3 before nothing, given backwards
        let pending = vec![place(1, Some(2)), place(2, Some(3)), place(3, None)];

        // When
        let (ordered, steps) = order(pending).expect("ordered");

        // Then
        let nodes: Vec<u64> = ordered.iter().map(|p| p.node.0).collect();
        assert_eq!(nodes, [3, 2, 1]);
        assert_eq!(steps, 8, "3 sorted, 2 released, 3 placed");
    }

    /// `count` placements, each before the next, given backwards.
    fn chain(count: u64) -> Vec<AxPlace> {
        (1..=count)
            .map(|n| place(n, (n < count).then_some(n + 1)))
            .collect()
    }

    #[test]
    fn a_chain_four_times_as_long_takes_at_most_four_and_a_half_times_the_steps() {
        // Given, When
        let (_, small) = order(chain(1000)).expect("ordered");
        let (_, large) = order(chain(4000)).expect("ordered");

        // Then: linear; the round-by-round form took 4000 x 4001 / 2
        assert!(large * 2 <= small * 9, "{small} steps, then {large}");
        assert!(small <= 3 * 1000, "{small}");
    }

    #[test]
    fn placements_waiting_on_each_other_are_refused_naming_them() {
        // Given
        let pending = vec![place(1, Some(2)), place(2, Some(1)), place(3, None)];

        // When
        let refused = order(pending).expect_err("a cycle");

        // Then
        assert_eq!(refused, AxError::PlacementCycle { nodes: vec![1, 2] });
    }
}

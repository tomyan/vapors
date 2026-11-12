//! The mirror's own model of where its elements are (R6 PR 5a,
//! review-v243 H2): each parent's children in order and each node's
//! parent, kept beside the DOM so a reconcile moves only the elements
//! that are out of place. Chromium resets a scroll container's offset
//! and drops DOM focus when an element is appended again, even in
//! place, so an element already where it belongs is left alone.

use std::collections::{HashMap, HashSet};

/// A move the DOM must make: `node` under `parent` before `before`, or
/// last when `before` is `None`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Move {
    pub(crate) node: u64,
    pub(crate) parent: Option<u64>,
    pub(crate) before: Option<u64>,
}

/// Each parent's children in order (`None`: the root's), and each
/// node's parent.
#[derive(Default)]
pub(crate) struct Order {
    children: HashMap<Option<u64>, Vec<u64>>,
    parent_of: HashMap<u64, Option<u64>>,
}

impl Order {
    /// Notes `node` placed under `parent` before `before`, or last.
    pub(crate) fn place(&mut self, node: u64, parent: Option<u64>, before: Option<u64>) {
        self.detach(node);
        let siblings = self.children.entry(parent).or_default();
        let at = before
            .and_then(|before| siblings.iter().position(|&n| n == before))
            .unwrap_or(siblings.len());
        siblings.insert(at, node);
        self.parent_of.insert(node, parent);
    }

    /// Notes `node` gone, with everything under it.
    pub(crate) fn remove(&mut self, node: u64) {
        self.detach(node);
        for child in self.children.remove(&Some(node)).unwrap_or_default() {
            self.remove(child);
        }
    }

    fn detach(&mut self, node: u64) {
        if let Some(parent) = self.parent_of.remove(&node)
            && let Some(siblings) = self.children.get_mut(&parent)
        {
            siblings.retain(|&n| n != node);
        }
    }

    /// `node`'s parent, and its parent's, up to the root.
    pub(crate) fn ancestors(&self, node: u64) -> impl Iterator<Item = u64> + '_ {
        let mut cursor = self.parent_of.get(&node).copied().flatten();
        std::iter::from_fn(move || {
            let next = cursor?;
            cursor = self.parent_of.get(&next).copied().flatten();
            Some(next)
        })
    }

    /// The moves that put `desired` (each node with its parent, in the
    /// order the children of each parent go) in place, leaving every
    /// element already there alone; the model then holds `desired`.
    /// Nodes not in `desired` must have been removed already.
    pub(crate) fn reconcile(&mut self, desired: &[(u64, Option<u64>)]) -> Vec<Move> {
        let mut wanted: HashMap<Option<u64>, Vec<u64>> = HashMap::new();
        for &(node, parent) in desired {
            wanted.entry(parent).or_default().push(node);
        }
        let mut moves = Vec::new();
        for (parent, nodes) in &wanted {
            let current = self.children.get(parent).cloned().unwrap_or_default();
            moves.extend(merge(*parent, nodes, &current));
        }
        self.children = wanted;
        self.parent_of = desired
            .iter()
            .map(|&(node, parent)| (node, parent))
            .collect();
        moves
    }
}

/// The moves that turn `current` into `wanted` under `parent`: a
/// wanted node is moved before the first current node not yet matched
/// (one that stays under this parent and is not placed ahead already),
/// or last, unless it is that node.
fn merge(parent: Option<u64>, wanted: &[u64], current: &[u64]) -> Vec<Move> {
    let stays: HashSet<u64> = wanted.iter().copied().collect();
    let mut moves = Vec::new();
    let mut placed_ahead: HashSet<u64> = HashSet::new();
    let mut at = 0;
    for &node in wanted {
        let skipped = |n: &u64| placed_ahead.contains(n) || !stays.contains(n);
        while at < current.len() && skipped(&current[at]) {
            at += 1;
        }
        if at < current.len() && current[at] == node {
            at += 1;
            continue;
        }
        moves.push(Move {
            node,
            parent,
            before: current.get(at).copied(),
        });
        placed_ahead.insert(node);
    }
    moves
}

#[cfg(test)]
#[path = "a11y_mirror_order_tests.rs"]
mod tests;

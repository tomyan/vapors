//! A lane's sum tree (`docs/plans/splice-layout.md`, sections 2 (b) and
//! 10.1): its children in tree order, each a leaf holding the child's
//! summary, each inner node the fold of its children's. A child is
//! named by a [`Handle`], which an insert or remove elsewhere does not
//! change; where it starts along the axis is the fold of the leaves to
//! its left, O(log n). A B-tree of fanout 16 with parent links, nodes
//! in a slab; a node left empty by a remove is freed, and an underfull
//! one is kept as it is.
//!
//! Keeping underfull nodes is a relaxed guard: a lane that peaked at N
//! children and holds n keeps up to about N/8 leaves (a leaf splits at
//! 17 into 8 and 9, and removes thin it to 1), so `handles`, a wide
//! `between` and the memory are proportional to the peak, not to n, and
//! the depth is the peak's. What would retire it: merging a node with a
//! neighbour when it falls below `FANOUT / 4`, or rebuilding the tree
//! when its nodes exceed `4 * n / FANOUT`; neither is built until a
//! workload shows a lane shrinking far below its peak.

use std::fmt::Debug;

/// A run of children folded: what a lane keeps per leaf and per node.
pub(crate) trait Summary: Copy + Debug + Default + PartialEq {
    /// `self` followed by `right`, `gap` between neighbouring children.
    /// Associative; `Default` is its identity.
    #[must_use]
    fn combine(&self, right: &Self, gap: f64) -> Self;
    /// Where `run`'s first child's border box starts, after `left`.
    fn start_of(left: &Self, run: &Self, gap: f64) -> f64;
    /// What `self`'s children reach, from its first child's start.
    fn bounds(&self) -> super::lane_bounds::Bounds;
}

/// A child of a lane, stable while it is in the tree, and refused once
/// its child is removed even if the index names a child put in since
/// (`lane_tree_handles.rs`).
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) struct Handle {
    index: u32,
    generation: u32,
}

/// The most children a node holds.
const FANOUT: usize = 16;

#[derive(Debug, Default)]
pub(super) struct Node<S> {
    pub(super) parent: Option<usize>,
    pub(super) leaf: bool,
    /// Handles in a leaf, node indices in an inner node.
    pub(super) children: Vec<u32>,
    pub(super) sums: Vec<S>,
}

/// A lane's children and their folds.
#[derive(Debug)]
pub(crate) struct LaneTree<S> {
    pub(super) nodes: Vec<Node<S>>,
    free_nodes: Vec<usize>,
    pub(super) root: usize,
    /// By handle index: the leaf holding its child and the generation
    /// the index is on.
    leaf_of: Vec<Slot>,
    free_handles: Vec<u32>,
    pub(super) gap: f64,
}

impl<S: Summary> LaneTree<S> {
    /// No children, `gap` between neighbours.
    pub(crate) fn new(gap: f64) -> Self {
        let root = Node {
            leaf: true,
            ..Node::default()
        };
        Self {
            nodes: vec![root],
            free_nodes: Vec::new(),
            root: 0,
            leaf_of: Vec::new(),
            free_handles: Vec::new(),
            gap,
        }
    }

    /// Puts a child with `summary` in after `previous`, or first.
    ///
    /// # Panics
    /// `previous` is not in the tree.
    pub(crate) fn insert_after(&mut self, previous: Option<Handle>, summary: S) -> Handle {
        let (leaf, at) = match previous {
            Some(previous) => {
                let (leaf, at) = self.locate(previous);
                (leaf, at + 1)
            }
            None => (self.first_leaf(), 0),
        };
        let handle = self.new_handle(leaf);
        let node = &mut self.nodes[leaf];
        node.children.insert(at, handle.index);
        node.sums.insert(at, summary);
        self.settle(leaf);
        handle
    }

    /// Takes `handle`'s child out.
    ///
    /// # Panics
    /// `handle` is not in the tree.
    pub(crate) fn remove(&mut self, handle: Handle) {
        let (leaf, at) = self.locate(handle);
        self.nodes[leaf].children.remove(at);
        self.nodes[leaf].sums.remove(at);
        self.forget(handle);
        self.prune(leaf);
    }

    /// Gives `handle`'s child `summary`.
    ///
    /// # Panics
    /// `handle` is not in the tree.
    pub(crate) fn set(&mut self, handle: Handle, summary: S) {
        let (leaf, at) = self.locate(handle);
        self.nodes[leaf].sums[at] = summary;
        self.refold(leaf);
    }

    /// `handle`'s child's summary.
    pub(crate) fn summary(&self, handle: Handle) -> S {
        let (leaf, at) = self.locate(handle);
        self.nodes[leaf].sums[at]
    }

    /// Every child folded.
    pub(crate) fn total(&self) -> S {
        self.fold(&self.nodes[self.root].sums)
    }

    /// Where `handle`'s child's border box starts along the axis.
    pub(crate) fn start(&self, handle: Handle) -> f64 {
        S::start_of(&self.before(handle), &self.summary(handle), self.gap)
    }

    /// The children before `handle`'s, folded.
    fn before(&self, handle: Handle) -> S {
        let (mut node, at) = self.locate(handle);
        let mut left = self.fold(&self.nodes[node].sums[..at]);
        while let Some(parent) = self.nodes[node].parent {
            let at = self.child_at(parent, node);
            left = self
                .fold(&self.nodes[parent].sums[..at])
                .combine(&left, self.gap);
            node = parent;
        }
        left
    }

    pub(super) fn fold(&self, sums: &[S]) -> S {
        sums.iter()
            .fold(S::default(), |run, sum| run.combine(sum, self.gap))
    }

    /// The index of `child` among `parent`'s children.
    fn child_at(&self, parent: usize, child: usize) -> usize {
        let children = &self.nodes[parent].children;
        let at = children.iter().position(|&c| c as usize == child);
        at.expect("a node's parent holds it")
    }

    fn first_leaf(&self) -> usize {
        let mut node = self.root;
        while !self.nodes[node].leaf {
            node = self.nodes[node].children[0] as usize;
        }
        node
    }

    /// Splits `node` while it is over [`FANOUT`], then refolds above.
    fn settle(&mut self, node: usize) {
        if self.nodes[node].children.len() > FANOUT {
            self.split(node);
        }
        self.refold(node);
    }

    /// Each ancestor of `node` given its child's new fold.
    fn refold(&mut self, mut node: usize) {
        while let Some(parent) = self.nodes[node].parent {
            let at = self.child_at(parent, node);
            self.nodes[parent].sums[at] = self.fold(&self.nodes[node].sums);
            node = parent;
        }
    }
}

#[path = "lane_tree_handles.rs"]
mod handles;
#[path = "lane_tree_shape.rs"]
mod shape;
use handles::Slot;

#[cfg(test)]
#[path = "lane_tree_random_tests.rs"]
mod random_tests;
#[cfg(test)]
#[path = "lane_tree_tests.rs"]
mod tests;

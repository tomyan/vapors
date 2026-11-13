//! The lane tree's shape: splitting a full node, freeing an emptied
//! one, the children in order, and the children an interval meets.

use super::{FANOUT, Handle, LaneTree, Node, Summary};

impl<S: Summary> LaneTree<S> {
    /// Every child, in tree order.
    pub(crate) fn handles(&self) -> Vec<Handle> {
        let mut out = Vec::new();
        let mut stack = vec![self.root];
        while let Some(node) = stack.pop() {
            let node = &self.nodes[node];
            if node.leaf {
                out.extend(node.children.iter().map(|&h| self.handle(h)));
            } else {
                stack.extend(node.children.iter().rev().map(|&c| c as usize));
            }
        }
        out
    }

    /// The children whose border box or overflow meets `from..=to`
    /// along the axis, in tree order: O(log n) plus those children and
    /// their nodes' siblings.
    pub(crate) fn between(&self, from: f64, to: f64) -> Vec<Handle> {
        let mut out = Vec::new();
        self.collect(self.root, S::default(), (from, to), &mut out);
        out
    }

    fn collect(&self, node: usize, mut left: S, (from, to): (f64, f64), out: &mut Vec<Handle>) {
        let node = &self.nodes[node];
        for (&child, sum) in node.children.iter().zip(&node.sums) {
            let start = S::start_of(&left, sum, self.gap);
            let bounds = sum.bounds();
            if start + bounds.low <= to && start + bounds.reach >= from {
                if node.leaf {
                    out.push(self.handle(child));
                } else {
                    self.collect(child as usize, left, (from, to), out);
                }
            }
            left = left.combine(sum, self.gap);
        }
    }

    /// Moves the second half of `node`'s children to a new node after
    /// it, splitting the parent in turn when that fills it.
    pub(super) fn split(&mut self, node: usize) {
        let half = self.nodes[node].children.len() / 2;
        let right = Node {
            parent: None,
            leaf: self.nodes[node].leaf,
            children: self.nodes[node].children.split_off(half),
            sums: self.nodes[node].sums.split_off(half),
        };
        let right = self.alloc(right);
        self.adopt(right);
        let parent = match self.nodes[node].parent {
            Some(parent) => parent,
            None => self.new_root(node),
        };
        self.attach_after(parent, node, right);
        if self.nodes[parent].children.len() > FANOUT {
            self.split(parent);
        }
    }

    /// Puts `right` into `parent` just after `node`, both freshly
    /// folded.
    fn attach_after(&mut self, parent: usize, node: usize, right: usize) {
        self.nodes[right].parent = Some(parent);
        let at = self.child_at(parent, node);
        let (left_sum, right_sum) = (self.fold_of(node), self.fold_of(right));
        let above = &mut self.nodes[parent];
        above.sums[at] = left_sum;
        above.children.insert(at + 1, to_u32(right));
        above.sums.insert(at + 1, right_sum);
    }

    /// Frees `node` and each ancestor a remove left without children,
    /// refolds above, and drops a root with one child.
    pub(super) fn prune(&mut self, mut node: usize) {
        while self.nodes[node].children.is_empty() && node != self.root {
            let parent = self.nodes[node]
                .parent
                .expect("a node other than the root has one");
            let at = self.child_at(parent, node);
            self.nodes[parent].children.remove(at);
            self.nodes[parent].sums.remove(at);
            self.release(node);
            node = parent;
        }
        self.refold(node);
        self.drop_single_roots();
    }

    /// While the root is an inner node with one child, that child is
    /// the root.
    fn drop_single_roots(&mut self) {
        while !self.nodes[self.root].leaf && self.nodes[self.root].children.len() == 1 {
            let child = self.nodes[self.root].children[0] as usize;
            self.release(self.root);
            self.nodes[child].parent = None;
            self.root = child;
        }
    }

    fn fold_of(&self, node: usize) -> S {
        self.fold(&self.nodes[node].sums)
    }

    /// A new root above `node`, which becomes its only child.
    fn new_root(&mut self, node: usize) -> usize {
        let root = Node {
            parent: None,
            leaf: false,
            children: vec![to_u32(node)],
            sums: vec![self.fold_of(node)],
        };
        let root = self.alloc(root);
        self.nodes[node].parent = Some(root);
        self.root = root;
        root
    }

    /// Points each of `node`'s children back at it.
    fn adopt(&mut self, node: usize) {
        for at in 0..self.nodes[node].children.len() {
            let child = self.nodes[node].children[at];
            if self.nodes[node].leaf {
                self.leaf_of[child as usize].leaf = Some(node);
            } else {
                self.nodes[child as usize].parent = Some(node);
            }
        }
    }

    fn alloc(&mut self, node: Node<S>) -> usize {
        if let Some(index) = self.free_nodes.pop() {
            self.nodes[index] = node;
            return index;
        }
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    fn release(&mut self, node: usize) {
        self.nodes[node] = Node::default();
        self.free_nodes.push(node);
    }
}

fn to_u32(node: usize) -> u32 {
    u32::try_from(node).expect("a lane under 2^32 nodes")
}

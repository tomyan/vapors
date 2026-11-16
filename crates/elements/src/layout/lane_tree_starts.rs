//! Walking a lane tree's starts in order (`lane_tree.rs`): from one
//! child on, each child with where it starts, one fold to reach the
//! first and one step a child after (`Summary::walk_step`).

use super::{Handle, LaneTree, Summary};

impl<S: Summary> LaneTree<S> {
    /// `handle`'s child and every child after it, in order, each with
    /// where it starts: O(log n) to reach it, then one step per child.
    #[cfg(test)]
    pub(crate) fn starts_from(&self, handle: Handle) -> Vec<(Handle, f64)> {
        let mut out = Vec::new();
        self.each_start_from(handle, &mut |index, start| {
            out.push((self.handle(index), start));
        });
        out
    }

    /// Calls `each` with `handle`'s child and every child after it, in
    /// order, each by its handle's index (`HandleSlots::at_index`) with
    /// where it starts, allocating nothing: the tree holds only its
    /// current children, so the index needs no generation check.
    pub(crate) fn each_start_from(&self, handle: Handle, each: &mut impl FnMut(u32, f64)) {
        let (leaf, at) = self.locate(handle);
        let mut walk = S::walk_from(&self.before(handle));
        self.starts_run_from(leaf, at, &mut walk, each);
        let mut node = leaf;
        while let Some(parent) = self.nodes[node].parent {
            let at = self.child_at(parent, node);
            self.starts_run_from(parent, at + 1, &mut walk, each);
            node = parent;
        }
    }

    /// Every child under `node` from its `at`th entry on, in order,
    /// `walk` stepped past each.
    fn starts_run_from(
        &self,
        node: usize,
        at: usize,
        walk: &mut S::Walk,
        each: &mut impl FnMut(u32, f64),
    ) {
        let node = &self.nodes[node];
        for (&child, sum) in node.children[at..].iter().zip(&node.sums[at..]) {
            if node.leaf {
                let (start, next) = S::walk_step(*walk, sum, self.gap);
                each(child, start);
                *walk = next;
            } else {
                self.starts_run_from(child as usize, 0, walk, each);
            }
        }
    }
}

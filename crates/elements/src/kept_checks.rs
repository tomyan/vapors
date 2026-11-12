//! The style phase's per-element checks, kept between updates (IU3
//! PR 3a): which elements the tree checks and the checks after styling
//! last refused. An update checks again only the elements marked since
//! (`style_checks`); when any element refuses, it collects the refusals
//! from a walk over the whole tree, in document order.

use std::collections::HashSet;

use vapors_dom::NodeId;

/// What the next update checks again, and what the last refused.
pub(crate) struct KeptChecks {
    /// Every element: none was checked yet, or the declared events, which
    /// every listener's check reads, changed.
    pub(crate) all: bool,
    /// Elements whose listeners changed; box marks cover the rest.
    pub(crate) marked: HashSet<NodeId>,
    /// Elements added to the tree, each with its subtree.
    pub(crate) added: Vec<NodeId>,
    /// Elements whose tags, attributes, text or listeners refuse.
    pub(crate) tree: HashSet<NodeId>,
    /// Elements whose `style` attribute does not parse.
    pub(crate) style_attribute: HashSet<NodeId>,
    /// Elements whose used style or media refuse.
    pub(crate) styled: HashSet<NodeId>,
}

impl Default for KeptChecks {
    fn default() -> Self {
        Self {
            all: true,
            marked: HashSet::new(),
            added: Vec::new(),
            tree: HashSet::new(),
            style_attribute: HashSet::new(),
            styled: HashSet::new(),
        }
    }
}

impl KeptChecks {
    /// Drops what is kept for `node`, which leaves the tree.
    pub(crate) fn forget(&mut self, node: NodeId) {
        self.marked.remove(&node);
        self.tree.remove(&node);
        self.style_attribute.remove(&node);
        self.styled.remove(&node);
    }

    /// Clears the marks, once the checks after styling ran for them.
    /// `marked` is a new set, not cleared: a mount's listeners mark every
    /// row, and `touched` iterates the capacity a cleared set keeps on
    /// every update (docs/plans/perf-finish-line.md, property 1).
    pub(crate) fn checked(&mut self) {
        self.all = false;
        self.marked = HashSet::new();
        self.added.clear();
    }

    /// Notes whether `node` refuses in `set`.
    pub(crate) fn note(set: &mut HashSet<NodeId>, node: NodeId, refuses: bool) {
        if refuses {
            set.insert(node);
        } else {
            set.remove(&node);
        }
    }
}

#[cfg(test)]
#[path = "kept_checks_tests.rs"]
mod tests;

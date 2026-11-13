//! Box marks (IU3): the elements whose boxes may have changed since
//! the last update, so the next rebuilds their boxes and keeps the
//! rest. Each mark resolves to its owner, the nearest element, itself
//! or an ancestor, that held a box slot last update and still makes a
//! box of its own; the owner's subtree is built again. A children mark
//! (`docs/plans/splice.md` (b)) builds only its element's own box and
//! the children it did not have (`owners.rs`).

use std::collections::HashSet;

use rustc_hash::FxHashMap;

use vapors_dom::NodeId;

use super::child_edits::ChildEdits;
pub(crate) use super::owners::{Build, Owner, Rebuild};
use super::owners::{Resolved, Resolver};
use super::slots::Placed;
use crate::arena::Arena;
use crate::edit_decline::EditDecline::MarkedInside;
use crate::style::UsedMap;

/// What changed since the last update: every box, the boxes of these
/// elements, or the children of these.
#[derive(Default)]
pub(crate) struct Marks {
    pub(crate) all: bool,
    nodes: HashSet<NodeId>,
    children: HashSet<NodeId>,
    /// The inserts and removes behind the children marks, where the
    /// document recorded them (`child_edits.rs`).
    pub(super) edits: FxHashMap<NodeId, ChildEdits>,
}

impl Marks {
    /// Marks `node`'s box, or the box of the element that owns it.
    pub(crate) fn mark(&mut self, node: NodeId) {
        self.nodes.insert(node);
    }

    /// Marks which children `node` has: one inserted, removed or
    /// moved.
    pub(crate) fn mark_children(&mut self, node: NodeId) {
        self.children.insert(node);
    }

    /// The marked nodes, either mark, freed and detached ones included.
    pub(crate) fn nodes(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.iter().chain(&self.children).copied()
    }

    /// What the next update builds again: every box when a mark
    /// reaches the root with no owner on the way (the root lost its
    /// box), or the owners under `root`, none inside another, in slot
    /// order so new keys take the same slots on every run. A mark on a
    /// node freed or detached since has no owner: the mark `remove` or
    /// `insert_before` put on its old parent covers its old boxes. A
    /// box mark is covered by a shallow owner that is its owner but not
    /// the node: the node is in a child that owner builds in full, or
    /// in its runs.
    pub(crate) fn rebuild(
        &self,
        arena: &Arena,
        used: &UsedMap,
        placed: &Placed,
        root: NodeId,
    ) -> Rebuild {
        let resolver = Resolver {
            arena,
            used,
            placed,
            root,
        };
        let (mut shallow, unowned) = self.shallow(&resolver);
        self.unedit_marked(&resolver, &mut shallow);
        let Some(full) = self.full(&resolver, &shallow) else {
            return Rebuild::Every;
        };
        shallow.retain(|node, _| !full.contains(node));
        Rebuild::Owners(resolver.outermost(&full, shallow), unowned)
    }

    /// The owners of the box marks, and of the children marks that are
    /// not `shallow`, but for the marks a shallow owner covers; none
    /// when a mark reaches the root with no owner.
    fn full(
        &self,
        resolver: &Resolver<'_>,
        shallow: &FxHashMap<NodeId, Build>,
    ) -> Option<HashSet<NodeId>> {
        let deep = self
            .children
            .iter()
            .filter(|node| !shallow.contains_key(node));
        let mut full = HashSet::new();
        for &node in self.nodes.iter().chain(deep) {
            match resolver.owner(node) {
                Resolved::Element(owner) if owner != node && shallow.contains_key(&owner) => {}
                Resolved::Element(owner) => _ = full.insert(owner),
                Resolved::None => return None,
                Resolved::Detached => {}
            }
        }
        Some(full)
    }

    /// Each edited owner a mark reaches outside the children its edit
    /// builds, built shallow instead: the edit keeps every other box,
    /// so it would keep the marked one as it was (review of #285, H1).
    fn unedit_marked(&self, resolver: &Resolver<'_>, shallow: &mut FxHashMap<NodeId, Build>) {
        for &node in self.nodes.iter().chain(&self.children) {
            let Resolved::Element(owner) = resolver.owner(node) else {
                continue;
            };
            let Some(Build::Edited(edits)) = shallow.get(&owner) else {
                continue;
            };
            if owner == node || edits.covers(resolver.arena, node, owner) {
                continue;
            }
            match resolver.kept_children(owner) {
                Some(kept) => _ = shallow.insert(owner, Build::Shallow(kept, Some(MarkedInside))),
                None => _ = shallow.remove(&owner),
            }
        }
    }
}

#[path = "marks_build.rs"]
mod build;

#[cfg(test)]
#[path = "marks_tests.rs"]
mod tests;

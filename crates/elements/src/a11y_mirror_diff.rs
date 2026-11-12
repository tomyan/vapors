//! A take after the first (R6): the nodes touched since the last take
//! and what each touch reaches (the node, its ancestors, and the nodes
//! naming it or an ancestor by id), each read again and compared with
//! the record last emitted.
//! Only a difference is emitted; a node that no longer qualifies is
//! removed, and one new or whose parent changed is placed.

use vapors_dom::NodeId;

use crate::a11y_error::AxError;
use crate::a11y_mirror::{AxMirrorNode, AxUpdate};
use crate::a11y_mirror_state::{AxIndex, REFERRING};
use crate::document::Document;
use crate::node_map::{NodeMap, NodeSet};

/// Where a node is after a take: out of the tree, or under a parent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum After {
    Gone,
    Under(Option<NodeId>),
}

/// What a take found.
#[derive(Default)]
pub(crate) struct Diff {
    pub(crate) nodes: Vec<AxMirrorNode>,
    pub(crate) removed: Vec<NodeId>,
    /// The nodes to place, each with its parent after the take.
    pub(crate) to_place: Vec<(NodeId, Option<NodeId>)>,
    /// Each node read again and still in the tree, with its parent.
    parents: NodeMap<Option<NodeId>>,
    removed_set: NodeSet,
}

impl Diff {
    fn added(&mut self, new: AxMirrorNode) {
        self.to_place.push((new.node, new.parent));
        self.parents.insert(new.node, new.parent);
        self.nodes.push(new);
    }

    fn kept(&mut self, old: &AxMirrorNode, new: AxMirrorNode, moved: bool) {
        if old.parent != new.parent || moved {
            self.to_place.push((new.node, new.parent));
        }
        self.parents.insert(new.node, new.parent);
        if *old != new {
            self.nodes.push(new);
        }
    }

    fn gone(&mut self, node: NodeId) {
        self.removed.push(node);
        self.removed_set.insert(node);
    }

    /// Where `node` is after the take.
    pub(crate) fn after(&self, node: NodeId, records: &NodeMap<AxMirrorNode>) -> After {
        if self.removed_set.contains(&node) {
            return After::Gone;
        }
        let parent = self
            .parents
            .get(&node)
            .copied()
            .or_else(|| records.get(&node).map(|record| record.parent));
        parent.map_or(After::Gone, After::Under)
    }
}

impl Document {
    /// What changed since the last take; `None` for nothing.
    pub(crate) fn ax_incremental(&mut self) -> Result<Option<AxUpdate>, AxError> {
        let focus = self.active_element();
        let (touched, replaced) = self.ax.take_touched();
        if touched.is_empty() && focus == self.ax.focus {
            return Ok(None);
        }
        let index = self.ax_index(&touched, &replaced);
        let candidates = self.ax_candidates(&touched, [self.ax.focus, focus], &index);
        let mut diff = Diff::default();
        for node in candidates {
            self.ax_classify(node, &index, &replaced, &mut diff)?;
        }
        let placed = self.ax_placements(&diff);
        let update = self.ax.apply(diff, placed, index, focus);
        Ok(Some(update).filter(|update| !update.is_empty()))
    }

    /// The index kept from the last take, unless a touch may have
    /// changed it: a node inserted or removed, or one with an `id` or a
    /// referring attribute.
    fn ax_index(&mut self, touched: &NodeSet, replaced: &NodeSet) -> AxIndex {
        let kept = self.ax.take_index();
        let stale = !replaced.is_empty() || touched.iter().any(|&node| self.ax_names_ids(node));
        match kept {
            Some(index) if !stale => index,
            _ => AxIndex::of(self),
        }
    }

    fn ax_names_ids(&self, node: NodeId) -> bool {
        if !self.contains(node) {
            return true;
        }
        self.tag(node).is_some()
            && (self.attribute(node, "id").is_some()
                || REFERRING
                    .iter()
                    .any(|attribute| self.attribute(node, attribute).is_some()))
    }

    /// Every node a touch reaches, and the nodes focus left and came to,
    /// each once.
    fn ax_candidates(
        &self,
        touched: &NodeSet,
        focus: [Option<NodeId>; 2],
        index: &AxIndex,
    ) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut seen = NodeSet::default();
        let mut add = |node: NodeId| {
            if seen.insert(node) {
                out.push(node);
            }
        };
        for &node in touched {
            self.ax_reach(node, index, &mut add);
        }
        for node in focus.into_iter().flatten() {
            add(node);
        }
        out
    }

    /// What a touch of `node` reaches: the node, its ancestors, and the
    /// nodes referring by id to it or to an ancestor.
    fn ax_reach(&self, node: NodeId, index: &AxIndex, add: &mut impl FnMut(NodeId)) {
        add(node);
        if !self.contains(node) {
            return;
        }
        let mut cursor = Some(node);
        while let Some(reached) = cursor {
            self.ax_referrers(reached, index, add);
            cursor = self.parent(reached);
            if let Some(parent) = cursor {
                add(parent);
            }
        }
    }

    fn ax_referrers(&self, node: NodeId, index: &AxIndex, add: &mut impl FnMut(NodeId)) {
        let Some(id) = self.tag(node).and_then(|_| self.attribute(node, "id")) else {
            return;
        };
        for &referrer in index.referrers.get(id).into_iter().flatten() {
            add(referrer);
        }
    }

    /// Reads `node` again and notes what differs from its record.
    fn ax_classify(
        &self,
        node: NodeId,
        index: &AxIndex,
        replaced: &NodeSet,
        diff: &mut Diff,
    ) -> Result<(), AxError> {
        let now = self.ax_record(node, index)?;
        match (self.ax.records.get(&node), now) {
            (None, None) => {}
            (None, Some(new)) => diff.added(new),
            (Some(old), Some(new)) => {
                let moved = self.ax_moved(node, new.parent, replaced);
                diff.kept(old, new, moved);
            }
            (Some(_), None) => diff.gone(node),
        }
        Ok(())
    }

    /// Whether `node`, or an element between it and `parent`, was
    /// inserted since the last take.
    fn ax_moved(&self, node: NodeId, parent: Option<NodeId>, replaced: &NodeSet) -> bool {
        let mut cursor = Some(node);
        while let Some(between) = cursor.filter(|&between| Some(between) != parent) {
            if replaced.contains(&between) {
                return true;
            }
            cursor = self.parent(between);
        }
        false
    }
}

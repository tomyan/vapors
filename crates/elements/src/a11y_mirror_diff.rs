//! A take after the first (R6): the nodes touched since the last take
//! and what each touch reaches (`a11y_mirror_reach`), each read again
//! and compared with the record last emitted. Only a difference is
//! emitted; a node that no longer qualifies is removed, and one new or
//! whose parent changed is placed. A take refused keeps what it took
//! for the next; focus on an element the tree leaves out is refused.

use vapors_dom::NodeId;

use crate::a11y_error::AxError;
use crate::a11y_mirror::{AxMirrorNode, AxPlace, AxUpdate};
use crate::a11y_mirror_index::AxIndex;
use crate::a11y_mirror_state::Touched;
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
    /// What changed since the last take; `None` for nothing. A refusal
    /// keeps what the take touched, so the next take holds it.
    pub(crate) fn ax_incremental(&mut self) -> Result<Option<AxUpdate>, AxError> {
        let focus = self.active_element();
        if self.ax.touched_nothing() && focus == self.ax.focus {
            return Ok(None);
        }
        let touched = self.ax.take_touched();
        let (index, renamed) = self.ax_index(&touched);
        match self.ax_diff(&touched, &renamed, &index, focus) {
            Ok((diff, placed)) => {
                let update = self.ax.apply(diff, placed, index, focus);
                Ok(Some(update).filter(|update| !update.is_empty()))
            }
            Err(refusal) => {
                self.ax.keep(touched, renamed, index);
                Err(refusal)
            }
        }
    }

    /// Every candidate read again, and the placements that follow.
    fn ax_diff(
        &self,
        touched: &Touched,
        renamed: &[NodeId],
        index: &AxIndex,
        focus: Option<NodeId>,
    ) -> Result<(Diff, Vec<AxPlace>), AxError> {
        let candidates = self.ax_candidates(touched, renamed, [self.ax.focus, focus], index);
        let mut diff = Diff::default();
        for node in candidates {
            self.ax_classify(node, index, &touched.replaced, &mut diff)?;
        }
        if let Some(node) = focus.filter(|&node| diff.after(node, &self.ax.records) == After::Gone)
        {
            return Err(self.ax_focus_outside(node));
        }
        let placed = self.ax_placements(&diff)?;
        Ok((diff, placed))
    }

    /// Focus is on `node`, which the tree leaves out.
    pub(crate) fn ax_focus_outside(&self, node: NodeId) -> AxError {
        AxError::FocusOutsideTree {
            tag: self.tag(node).unwrap_or("#text").to_owned(),
        }
    }

    /// The index for this take: the one kept, patched for the nodes
    /// touched, or rebuilt by a scan when the tree's shape changed (an
    /// element inserted or removed: its hidden descendants carry ids
    /// without boxes, so no touch names them) or a patch could not be
    /// certain. With it, the nodes named by an id whose element changed.
    fn ax_index(&mut self, touched: &Touched) -> (AxIndex, Vec<NodeId>) {
        let Some(mut kept) = self.ax.take_index() else {
            return (self.ax_scan(), Vec::new());
        };
        if !self.ax_shape_changed(touched)
            && let Some(changed) = kept.patch(self, &touched.nodes)
        {
            let renamed = kept.referrers_of(&changed);
            return (kept, renamed);
        }
        let index = self.ax_scan();
        let changed = kept.changed_ids(&index);
        let mut renamed = kept.referrers_of(&changed);
        renamed.extend(index.referrers_of(&changed));
        (index, renamed)
    }

    fn ax_shape_changed(&self, touched: &Touched) -> bool {
        !touched.replaced.is_empty() || touched.nodes.iter().any(|&node| !self.contains(node))
    }

    /// The index rebuilt by a scan of the document, counted.
    pub(crate) fn ax_scan(&mut self) -> AxIndex {
        self.ax.scans += 1;
        AxIndex::of(self)
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
}

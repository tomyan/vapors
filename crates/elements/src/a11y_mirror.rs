//! The a11y update a mirror takes (R6, `docs/plans/r6.md`): what
//! changed in the tree since the last take, as records of the nodes
//! new or changed, the nodes removed, placements for the nodes new or
//! moved, and the focused node when it moved. The first take holds
//! every node (`every`), in document order, each with its parent; a
//! mirror builds itself from it by appending each to its parent. After
//! that the cost of a take follows what was touched
//! (`a11y_mirror_state`), never the page. A take before an update is
//! refused, as the driver's tree is. The types are `vapors-a11y`'s, so
//! a platform's mirror takes them without this crate.

use crate::a11y_error::AxError;
use crate::a11y_mirror_counts::AxTakeCounts;
use crate::a11y_mirror_index::AxIndex;
use crate::a11y_mirror_record::AboveMemo;
use crate::document::Document;

pub use vapors_a11y::{AxMirrorNode, AxPlace, AxRange, AxUpdate};

impl Document {
    /// What changed in the tree since the last take; `None` when
    /// nothing did. The first take holds every node.
    ///
    /// # Errors
    /// [`AxError::Owned`]: a consumer is attached
    /// ([`Document::ax_mirror_attach`]), and the take is its.
    /// [`AxError::NotUpdated`]: the document has changed since its last
    /// update, or has none. [`AxError::Unexposable`]: a node ARIA
    /// requires a state for that the tree does not carry.
    /// [`AxError::NotFinite`]: a node's bounds are not finite.
    /// [`AxError::FocusOutsideTree`]: focus is on an element the tree
    /// leaves out. A refused take keeps what it took for the next.
    pub fn take_ax_update(&mut self) -> Result<Option<AxUpdate>, AxError> {
        self.ax.owner.none()?;
        self.take_ax_update_unowned()
    }

    /// The take, whoever asks.
    pub(crate) fn take_ax_update_unowned(&mut self) -> Result<Option<AxUpdate>, AxError> {
        if self.painted.is_none() || self.changes.since_update {
            return Err(AxError::NotUpdated);
        }
        self.ax.counts = AxTakeCounts::default();
        if self.ax.all {
            return self.ax_rebuild().map(Some);
        }
        self.ax_incremental()
    }

    /// Every node, in document order.
    fn ax_rebuild(&mut self) -> Result<AxUpdate, AxError> {
        let index = self.ax_scan();
        let nodes = self.ax_every(&index)?;
        self.ax.counts.nodes_read = nodes.len();
        let focus = self.active_element();
        if let Some(node) = focus.filter(|&node| !nodes.iter().any(|n| n.node == node)) {
            return Err(self.ax_focus_outside(node));
        }
        self.ax.reset(&nodes, index, focus);
        Ok(AxUpdate {
            every: true,
            nodes,
            focus: Some(focus),
            ..AxUpdate::default()
        })
    }

    /// Every node as a first take would hold it now, without taking:
    /// what a mirror kept in step by the takes since must hold.
    ///
    /// # Errors
    /// As [`Document::take_ax_update`].
    pub fn ax_mirror_snapshot(&self) -> Result<Vec<AxMirrorNode>, AxError> {
        if self.painted.is_none() || self.changes.since_update {
            return Err(AxError::NotUpdated);
        }
        self.ax_every(&AxIndex::of(self))
    }

    /// Every node's record, in document order: one pass, each element's
    /// parent in the tree carried down from its own parent's entry.
    pub(crate) fn ax_every(&self, index: &AxIndex) -> Result<Vec<AxMirrorNode>, AxError> {
        let mut nodes = Vec::new();
        let mut memo = AboveMemo::default();
        for node in self.elements() {
            let over = memo.over(self, node);
            let record = self.ax_record_under(node, index, over)?;
            memo.note(self, node, over, record.is_some());
            nodes.extend(record);
        }
        Ok(nodes)
    }
}

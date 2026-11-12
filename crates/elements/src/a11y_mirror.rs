//! The a11y update a mirror takes (R6, `docs/plans/r6.md`): the nodes
//! of the tree as records, each with its parent, in document order. A
//! mirror builds itself from a take of every node by appending each
//! record to its parent. This take is every node every time; the next
//! slice holds only what changed since the last take (`removed`,
//! `placed` and `focus` are its fields). A take before an update is
//! refused, as the driver's tree is.

use vapors_dom::NodeId;

use crate::a11y_error::AxError;
use crate::a11y_mirror_record::AboveMemo;
use crate::a11y_mirror_state::AxIndex;
use crate::a11y_tree::AxStates;
use crate::document::Document;
use crate::layout::BoxRect;

/// What changed since the last take.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AxUpdate {
    /// The first take, or every box was built: the mirror is rebuilt
    /// from `nodes`, which then holds every node and `placed` nothing.
    pub every: bool,
    /// Nodes no longer in the tree.
    pub removed: Vec<NodeId>,
    /// Nodes new, or whose record changed.
    pub nodes: Vec<AxMirrorNode>,
    /// Nodes new or moved, in an order a mirror applies one by one:
    /// a placement's `before` is already in place when it is reached.
    pub placed: Vec<AxPlace>,
    /// The focused node, when focus moved; `Some(None)` when it left.
    pub focus: Option<Option<NodeId>>,
}

/// One node as a mirror shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct AxMirrorNode {
    /// The element.
    pub node: NodeId,
    /// The nearest node above it in the tree; `None` for a top node.
    pub parent: Option<NodeId>,
    /// Its role, the ARIA token.
    pub role: String,
    /// Its accessible name; empty for none.
    pub name: String,
    /// Its value, for a control that shows one, when not empty.
    pub value: Option<String>,
    /// Its accessible description, when not empty.
    pub description: Option<String>,
    /// Its states.
    pub states: AxStates,
    /// A slider's or spinbutton's range.
    pub range: Option<AxRange>,
    /// Whether it can take focus now (`focus_candidate`).
    pub focusable: bool,
    /// Its border box in CSS pixels, as the last update placed it.
    pub bounds: BoxRect,
}

/// A ranged control's bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxRange {
    /// `aria-valuemin`.
    pub min: f64,
    /// `aria-valuemax`.
    pub max: f64,
}

/// Where a node goes: under `parent`, before `before`, or last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AxPlace {
    /// The node.
    pub node: NodeId,
    /// Its parent in the tree; `None` for a top node.
    pub parent: Option<NodeId>,
    /// The sibling it goes before, which the mirror holds by then;
    /// `None` to go last.
    pub before: Option<NodeId>,
}

impl AxUpdate {
    /// Whether the take found nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.every
            && self.removed.is_empty()
            && self.nodes.is_empty()
            && self.placed.is_empty()
            && self.focus.is_none()
    }
}

impl Document {
    /// The tree for a mirror: every node, in document order.
    ///
    /// # Errors
    /// [`AxError::NotUpdated`]: the document has changed since its last
    /// update, or has none. [`AxError::Unexposable`]: a node ARIA
    /// requires a state for that the tree does not carry.
    /// [`AxError::NotFinite`]: a node's bounds are not finite.
    pub fn take_ax_update(&mut self) -> Result<Option<AxUpdate>, AxError> {
        if self.painted.is_none() || self.changes.since_update {
            return Err(AxError::NotUpdated);
        }
        self.ax_rebuild().map(Some)
    }

    /// Every node, in document order.
    fn ax_rebuild(&mut self) -> Result<AxUpdate, AxError> {
        let index = AxIndex::of(self);
        let nodes = self.ax_every(&index)?;
        let focus = self.active_element();
        self.ax.reset(&nodes, index, focus);
        Ok(AxUpdate {
            every: true,
            nodes,
            focus: Some(focus),
            ..AxUpdate::default()
        })
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

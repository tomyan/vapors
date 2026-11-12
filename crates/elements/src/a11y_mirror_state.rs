//! What the a11y mirror keeps between takes (R6): the records last
//! emitted, the nodes touched since, and the index of ids. A node is
//! touched where layout reports its box built, moved or freed
//! (`touch_slots`): every setter marks the box of the element it
//! changes, so the boxes built again after an update are the elements
//! changed, each one, and the boxes freed are the elements gone or
//! hidden. An element inserted is noted as moved too
//! (`Document::note_replaced`): its box may be built the same at a new
//! place. Nothing is collected until the first take, which holds every
//! node; every box built again makes the next take hold every node too.
//! A refused take is its own state: what it took is kept for the next
//! (`keep`), so a refusal loses no change.

use vapors_dom::NodeId;

use crate::a11y_mirror::{AxMirrorNode, AxPlace, AxUpdate};
use crate::a11y_mirror_counts::AxTakeCounts;
use crate::a11y_mirror_diff::Diff;
use crate::a11y_mirror_index::AxIndex;
use crate::a11y_mirror_owner::Owner;
use crate::changed_slots::ChangedSlots;
use crate::document::Document;
use crate::layout::{Key, Slots};
use crate::node_map::{NodeMap, NodeSet};

/// What a take collects before it reads anything.
#[derive(Default)]
pub(crate) struct Touched {
    /// The nodes whose boxes were built, moved or freed since the last
    /// take.
    pub(crate) nodes: NodeSet,
    /// Nodes inserted since the last take: moved among their siblings.
    pub(crate) replaced: NodeSet,
    /// Nodes a refused take found named by an id whose element changed,
    /// to read again.
    pub(crate) renamed: NodeSet,
}

pub(crate) struct AxMirror {
    /// The next take holds every node: no take yet, or every box built.
    pub(crate) all: bool,
    touched: Touched,
    /// The records last emitted, by node.
    pub(crate) records: NodeMap<AxMirrorNode>,
    /// The focused node as last emitted.
    pub(crate) focus: Option<NodeId>,
    index: Option<AxIndex>,
    /// What the last take did.
    pub(crate) counts: AxTakeCounts,
    /// The consumer that owns the takes, if attached.
    pub(crate) owner: Owner,
}

impl Default for AxMirror {
    fn default() -> Self {
        Self {
            all: true,
            touched: Touched::default(),
            records: NodeMap::default(),
            focus: None,
            index: None,
            counts: AxTakeCounts::default(),
            owner: Owner::default(),
        }
    }
}

impl AxMirror {
    /// Notes the boxes an update built, moved or freed, by their nodes.
    pub(crate) fn touch_slots(&mut self, changed: &ChangedSlots, slots: &Slots) {
        // a box made or unmade a containing block, or a positioned box
        // freed, can change which parent node a node's box is placed in
        // (hoisted or not) without moving it, so nothing touches it:
        // every node is read again
        if changed.every || changed.stackings.restacked {
            self.all = true;
            self.touched = Touched::default();
        }
        if self.all {
            return;
        }
        let built = changed.rebuilt.iter().chain(&changed.moved);
        for key in built.filter_map(|&slot| slots.key(slot)) {
            self.touched.nodes.insert(node_of(key));
        }
        for (_, key) in &changed.freed {
            self.touched.nodes.insert(node_of(*key));
        }
    }

    /// Notes `node` scrolled: its offset changed, which moves no box
    /// (its content is in its own scroll frame, IU4).
    /// An offset clamped by an update is read without this: the boxes
    /// built, moved or freed inside it reach it as their ancestor.
    pub(crate) fn touch_node(&mut self, node: NodeId) {
        if !self.all {
            self.touched.nodes.insert(node);
        }
    }

    /// Notes `node` inserted: changed, and moved among its siblings.
    fn touch_replaced(&mut self, node: NodeId) {
        if self.all {
            return;
        }
        self.touched.nodes.insert(node);
        self.touched.replaced.insert(node);
    }

    /// Whether nothing was touched since the last take.
    pub(crate) fn touched_nothing(&self) -> bool {
        self.touched.nodes.is_empty() && self.touched.renamed.is_empty()
    }

    /// What was touched since the last take; cleared.
    pub(crate) fn take_touched(&mut self) -> Touched {
        std::mem::take(&mut self.touched)
    }

    /// The index kept from the last take, if any.
    pub(crate) fn take_index(&mut self) -> Option<AxIndex> {
        self.index.take()
    }

    /// Keeps what a refused take took, for the next take: what it
    /// touched, the nodes it found renamed, and the index it read.
    pub(crate) fn keep(&mut self, mut touched: Touched, renamed: Vec<NodeId>, index: AxIndex) {
        touched.renamed.extend(renamed);
        touched.nodes.extend(self.touched.nodes.drain());
        touched.replaced.extend(self.touched.replaced.drain());
        touched.renamed.extend(self.touched.renamed.drain());
        self.touched = touched;
        self.index = Some(index);
    }

    /// Keeps `nodes` as the records of every node, after a take of all.
    pub(crate) fn reset(&mut self, nodes: &[AxMirrorNode], index: AxIndex, focus: Option<NodeId>) {
        self.records = nodes.iter().map(|n| (n.node, n.clone())).collect();
        self.touched = Touched::default();
        self.all = false;
        self.index = Some(index);
        self.focus = focus;
    }

    /// Applies a take's findings to the records; the update it emits.
    pub(crate) fn apply(
        &mut self,
        diff: Diff,
        placed: Vec<AxPlace>,
        index: AxIndex,
        focus: Option<NodeId>,
    ) -> AxUpdate {
        for node in &diff.removed {
            self.records.remove(node);
        }
        for record in &diff.nodes {
            self.records.insert(record.node, record.clone());
        }
        let moved = (focus != self.focus).then_some(focus);
        self.focus = focus;
        self.index = Some(index);
        AxUpdate {
            every: false,
            removed: diff.removed,
            nodes: diff.nodes,
            placed,
            focus: moved,
        }
    }
}

/// The element a box's key names, or the text's element for a text run.
fn node_of(key: Key) -> NodeId {
    match key {
        Key::Element(node) | Key::Text(node, _) => node,
    }
}

impl Document {
    /// Notes `node` inserted into the tree, for
    /// [`Document::take_changed`] and for the a11y mirror.
    pub(crate) fn note_replaced(&mut self, node: NodeId) {
        self.note_change();
        self.ax.touch_replaced(node);
    }
}

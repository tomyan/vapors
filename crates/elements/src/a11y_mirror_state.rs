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

use std::collections::HashMap;

use vapors_dom::NodeId;

use crate::a11y_mirror::{AxMirrorNode, AxPlace, AxUpdate};
use crate::a11y_mirror_diff::Diff;
use crate::a11y_roles::Ids;
use crate::changed_slots::ChangedSlots;
use crate::document::Document;
use crate::layout::{Key, Slots};
use crate::node_map::{NodeMap, NodeSet};

/// The attributes naming other elements by id, whose owner's name or
/// description changes with the named element's content.
pub(crate) const REFERRING: [&str; 2] = ["aria-labelledby", "aria-describedby"];

pub(crate) struct AxMirror {
    /// The next take holds every node: no take yet, or every box built.
    pub(crate) all: bool,
    /// The nodes whose boxes were built, moved or freed since the last
    /// take.
    touched: NodeSet,
    /// Nodes inserted since the last take: moved among their siblings.
    replaced: NodeSet,
    /// The records last emitted, by node.
    pub(crate) records: NodeMap<AxMirrorNode>,
    /// The focused node as last emitted.
    pub(crate) focus: Option<NodeId>,
    index: Option<AxIndex>,
}

impl Default for AxMirror {
    fn default() -> Self {
        Self {
            all: true,
            touched: NodeSet::default(),
            replaced: NodeSet::default(),
            records: NodeMap::default(),
            focus: None,
            index: None,
        }
    }
}

/// Each id to its first element, and each id to the elements naming it
/// in a [`REFERRING`] attribute.
pub(crate) struct AxIndex {
    pub(crate) ids: Ids,
    pub(crate) referrers: HashMap<String, Vec<NodeId>>,
}

impl AxIndex {
    /// The index of `doc`'s elements now.
    pub(crate) fn of(doc: &Document) -> Self {
        let mut index = Self {
            ids: Ids::new(),
            referrers: HashMap::new(),
        };
        for node in doc.elements() {
            if let Some(id) = doc.attribute(node, "id") {
                index.ids.entry(id.to_owned()).or_insert(node);
            }
            for attribute in REFERRING {
                let named = doc.attribute(node, attribute).unwrap_or_default();
                for id in named.split_whitespace() {
                    index.referrers.entry(id.to_owned()).or_default().push(node);
                }
            }
        }
        index
    }
}

impl AxMirror {
    /// Notes the boxes an update built, moved or freed, by their nodes.
    pub(crate) fn touch_slots(&mut self, changed: &ChangedSlots, slots: &Slots) {
        if changed.every {
            self.all = true;
            self.touched.clear();
            self.replaced.clear();
        }
        if self.all {
            return;
        }
        let built = changed.rebuilt.iter().chain(&changed.moved);
        for key in built.filter_map(|&slot| slots.key(slot)) {
            self.touched.insert(node_of(key));
        }
        for (_, key) in &changed.freed {
            self.touched.insert(node_of(*key));
        }
    }

    /// Notes `node` inserted: changed, and moved among its siblings.
    fn touch_replaced(&mut self, node: NodeId) {
        if self.all {
            return;
        }
        self.touched.insert(node);
        self.replaced.insert(node);
    }

    /// The nodes touched and the nodes inserted since the last take;
    /// both cleared.
    pub(crate) fn take_touched(&mut self) -> (NodeSet, NodeSet) {
        (
            std::mem::take(&mut self.touched),
            std::mem::take(&mut self.replaced),
        )
    }

    /// Keeps `nodes` as the records of every node, after a take of all.
    pub(crate) fn reset(&mut self, nodes: &[AxMirrorNode], index: AxIndex, focus: Option<NodeId>) {
        self.records = nodes.iter().map(|n| (n.node, n.clone())).collect();
        self.touched.clear();
        self.replaced.clear();
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

    /// The index kept from the last take, if any.
    pub(crate) fn take_index(&mut self) -> Option<AxIndex> {
        self.index.take()
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

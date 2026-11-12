//! A mirror as the tests model one (R6): the records held, each parent's
//! children in order and the focused node, built from the first take
//! and kept in step by the takes after, as the web mirror keeps its DOM
//! (`docs/plans/r6.md`): records first, then placements in their order,
//! then removals, then focus. `assert_mirrors` holds it against what a
//! first take would hold now (`Document::ax_mirror_snapshot`), so a take
//! that lost a change is caught whatever else the test looks at.

use std::collections::HashMap;

use vapors_dom::NodeId;
use vapors_elements::{AxMirrorNode, AxUpdate, Document};

pub struct Model {
    records: HashMap<NodeId, AxMirrorNode>,
    children: HashMap<Option<NodeId>, Vec<NodeId>>,
    parent_of: HashMap<NodeId, Option<NodeId>>,
    focus: Option<NodeId>,
}

impl Model {
    /// A mirror built from a take of every node: each appended to its
    /// parent, in order.
    pub fn from_every(update: &AxUpdate) -> Self {
        assert!(update.every, "the model starts from every node");
        let mut model = Self {
            records: HashMap::new(),
            children: HashMap::new(),
            parent_of: HashMap::new(),
            focus: None,
        };
        model.apply(update);
        model
    }

    /// Applies `update`.
    pub fn apply(&mut self, update: &AxUpdate) {
        if update.every {
            self.records.clear();
            self.children.clear();
            self.parent_of.clear();
        }
        for record in &update.nodes {
            self.records.insert(record.node, record.clone());
            if update.every {
                self.attach(record.node, record.parent, None);
            }
        }
        for place in &update.placed {
            self.detach(place.node);
            self.attach(place.node, place.parent, place.before);
        }
        for &node in &update.removed {
            self.detach(node);
            self.remove_subtree(node);
        }
        if let Some(focus) = update.focus {
            self.focus = focus;
        }
    }

    /// `parent`'s children, in order.
    pub fn children(&self, parent: Option<NodeId>) -> Vec<NodeId> {
        self.children.get(&parent).cloned().unwrap_or_default()
    }

    /// The model holds what a first take of `doc` would hold now, node
    /// for node, and the node `doc` focuses.
    pub fn assert_mirrors(&self, doc: &Document) {
        let truth = doc.ax_mirror_snapshot().expect("an updated document");
        let held = self.flatten(None);
        let lines = |records: &[AxMirrorNode]| records.iter().map(line).collect::<Vec<_>>();
        assert_eq!(
            lines(&held),
            lines(&truth),
            "the model diverged from what a first take holds"
        );
        assert_eq!(self.focus, doc.active_element(), "the model's focus");
    }

    /// The records in the model's order, each parent before its children.
    fn flatten(&self, parent: Option<NodeId>) -> Vec<AxMirrorNode> {
        let mut out = Vec::new();
        for node in self.children(parent) {
            let record = &self.records[&node];
            assert_eq!(
                record.parent, parent,
                "{node:?}'s record names another parent"
            );
            out.push(record.clone());
            out.extend(self.flatten(Some(node)));
        }
        out
    }

    fn attach(&mut self, node: NodeId, parent: Option<NodeId>, before: Option<NodeId>) {
        let siblings = self.children.entry(parent).or_default();
        let at = match before {
            Some(before) => siblings
                .iter()
                .position(|&s| s == before)
                .unwrap_or_else(|| {
                    panic!("{node:?} placed before {before:?}, not under {parent:?}")
                }),
            None => siblings.len(),
        };
        siblings.insert(at, node);
        self.parent_of.insert(node, parent);
    }

    fn detach(&mut self, node: NodeId) {
        if let Some(parent) = self.parent_of.remove(&node)
            && let Some(siblings) = self.children.get_mut(&parent)
        {
            siblings.retain(|&s| s != node);
        }
    }

    /// Drops `node`'s record and its subtree's, as removing a DOM element
    /// drops what it holds.
    fn remove_subtree(&mut self, node: NodeId) {
        self.records.remove(&node);
        for child in self.children.remove(&Some(node)).unwrap_or_default() {
            self.parent_of.remove(&child);
            self.remove_subtree(child);
        }
    }
}

/// A record as one line, for a readable difference.
fn line(record: &AxMirrorNode) -> String {
    let b = record.bounds;
    format!(
        "{:?} {} {:?} v={:?} d={:?} {:?} range={:?} focusable={} ({}, {}, {}, {}) under {:?}",
        record.node,
        record.role,
        record.name,
        record.value,
        record.description,
        record.states.set(),
        record.range,
        record.focusable,
        b.x,
        b.y,
        b.width,
        b.height,
        record.parent
    )
}

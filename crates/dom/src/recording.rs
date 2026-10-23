//! A [`Dom`] for tests: records every call and keeps the tree so a
//! test can read it back as HTML and dispatch events into it.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use crate::{Handler, NodeId, PropertyValue};

mod dispatch;
mod dom_impl;
mod html;

/// One recorded call, with the ids involved.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)] // the variants mirror the Dom methods one to one
pub enum Call {
    CreateElement {
        node: NodeId,
        tag: String,
    },
    CreateText {
        node: NodeId,
        text: String,
    },
    SetAttribute {
        node: NodeId,
        name: String,
        value: String,
    },
    SetText {
        node: NodeId,
        text: String,
    },
    SetProperty {
        node: NodeId,
        name: String,
        value: PropertyValue,
    },
    Append {
        parent: NodeId,
        child: NodeId,
    },
    InsertBefore {
        parent: NodeId,
        child: NodeId,
        before: Option<NodeId>,
    },
    Remove {
        node: NodeId,
    },
    Listen {
        node: NodeId,
        event: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum Kind {
    Element {
        tag: String,
        attributes: BTreeMap<String, String>,
        properties: BTreeMap<String, PropertyValue>,
    },
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
struct Record {
    kind: Kind,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
}

/// The recording fake. Node 0 is the root the test mounts into.
#[derive(Default)]
pub struct RecordingDom {
    calls: Vec<Call>,
    nodes: Vec<Record>,
    listeners: HashMap<(NodeId, String), Vec<Rc<RefCell<Handler>>>>,
}

impl RecordingDom {
    /// An empty tree with just the root.
    #[must_use]
    pub fn new() -> RecordingDom {
        let mut dom = RecordingDom::default();
        dom.push(Kind::Element {
            tag: "root".into(),
            attributes: BTreeMap::new(),
            properties: BTreeMap::new(),
        });
        dom
    }

    /// The node everything is mounted into.
    #[must_use]
    pub fn root(&self) -> NodeId {
        NodeId(0)
    }

    /// Every call so far, in order.
    #[must_use]
    pub fn calls(&self) -> &[Call] {
        &self.calls
    }

    /// Forgets the calls so far, to watch the next step alone.
    pub fn clear_calls(&mut self) {
        self.calls.clear();
    }

    /// The children of a node, in order.
    #[must_use]
    pub fn children(&self, node: NodeId) -> Vec<NodeId> {
        self.record(node).children.clone()
    }

    /// A property set on an element.
    #[must_use]
    pub fn property(&self, node: NodeId, name: &str) -> Option<PropertyValue> {
        match &self.record(node).kind {
            Kind::Element { properties, .. } => properties.get(name).cloned(),
            Kind::Text(_) => None,
        }
    }

    fn push(&mut self, kind: Kind) -> NodeId {
        let id = NodeId(u64::try_from(self.nodes.len()).expect("fewer than 2^64 nodes"));
        self.nodes.push(Record {
            kind,
            parent: None,
            children: Vec::new(),
        });
        id
    }

    fn record(&self, node: NodeId) -> &Record {
        self.nodes
            .get(usize::try_from(node.0).expect("ids are indices"))
            .unwrap_or_else(|| panic!("{node:?} was never created by this RecordingDom"))
    }

    fn record_mut(&mut self, node: NodeId) -> &mut Record {
        self.nodes
            .get_mut(usize::try_from(node.0).expect("ids are indices"))
            .unwrap_or_else(|| panic!("{node:?} was never created by this RecordingDom"))
    }

    fn detach(&mut self, node: NodeId) {
        if let Some(parent) = self.record_mut(node).parent.take() {
            self.record_mut(parent)
                .children
                .retain(|child| *child != node);
        }
    }

    fn place(&mut self, parent: NodeId, child: NodeId, before: Option<NodeId>) {
        self.detach(child);
        let siblings = &mut self.record_mut(parent).children;
        let at = before
            .and_then(|before| siblings.iter().position(|sibling| *sibling == before))
            .unwrap_or(siblings.len());
        siblings.insert(at, child);
        self.record_mut(child).parent = Some(parent);
    }
}

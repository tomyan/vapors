//! A node of the element tree: an element or a text run, and its links
//! to parent and siblings.

use std::collections::BTreeMap;

use vapors_dom::{Handler, NodeId, PropertyValue};

/// What a node is.
pub(crate) enum Kind {
    /// An element with its tag, attributes, properties and listeners.
    Element(Element),
    /// A text node.
    Text(String),
}

/// An element's own data.
pub(crate) struct Element {
    pub(crate) tag: String,
    pub(crate) attributes: BTreeMap<String, String>,
    pub(crate) properties: BTreeMap<String, PropertyValue>,
    pub(crate) listeners: Vec<(String, Handler)>,
}

impl Element {
    pub(crate) fn new(tag: &str) -> Self {
        Self {
            tag: tag.to_owned(),
            attributes: BTreeMap::new(),
            properties: BTreeMap::new(),
            listeners: Vec::new(),
        }
    }
}

/// A node and its links. Siblings are a doubly linked list so moves and
/// removals are O(1).
pub(crate) struct Node {
    pub(crate) kind: Kind,
    pub(crate) parent: Option<NodeId>,
    pub(crate) first_child: Option<NodeId>,
    pub(crate) last_child: Option<NodeId>,
    pub(crate) previous: Option<NodeId>,
    pub(crate) next: Option<NodeId>,
}

impl Node {
    pub(crate) fn new(kind: Kind) -> Self {
        Self {
            kind,
            parent: None,
            first_child: None,
            last_child: None,
            previous: None,
            next: None,
        }
    }

    pub(crate) fn element(&self) -> Option<&Element> {
        match &self.kind {
            Kind::Element(element) => Some(element),
            Kind::Text(_) => None,
        }
    }
}

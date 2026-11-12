//! A node of the element tree: an element or a text run, and its links
//! to parent and siblings.

use std::collections::BTreeMap;

use vapors_css::tree::States;
use vapors_dom::{NodeId, PropertyValue, StyleKey};

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
    /// Shared so dispatch can call them with the document unborrowed.
    pub(crate) listeners: Vec<(String, vapors_dom::Listener)>,
    /// The pointer's flags: `:hover` and `:active`.
    pub(crate) states: States,
    /// The template's key, for an element `create_keyed_element` made.
    pub(crate) key: Option<&'static StyleKey>,
    /// Which of the key's `class:` directives are on, as bits.
    pub(crate) toggles: u64,
}

impl Element {
    pub(crate) fn new(tag: &str) -> Self {
        Self {
            tag: tag.to_owned(),
            attributes: BTreeMap::new(),
            properties: BTreeMap::new(),
            listeners: Vec::new(),
            states: States::NONE,
            key: None,
            toggles: 0,
        }
    }

    /// An element of `key`'s tag with the key's attributes set.
    pub(crate) fn keyed(key: &'static StyleKey) -> Self {
        let mut element = Self::new(key.tag);
        for (name, value) in key.attributes() {
            element.attributes.insert(name.to_owned(), value);
        }
        element.key = Some(key);
        element
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

    pub(crate) fn element_mut(&mut self) -> Option<&mut Element> {
        match &mut self.kind {
            Kind::Element(element) => Some(element),
            Kind::Text(_) => None,
        }
    }
}

//! A small element tree that implements [`Element`](crate::selector::Element):
//! the shape the tests and the `css_dump` example use, and a reference for
//! the element layer that will implement the trait on its own tree.

mod node;

use std::collections::BTreeMap;

pub use node::Node;

/// Index of a node in a [`Tree`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(usize);

/// The state flags a selector's state pseudo-classes read. The tree layer
/// sets them; the engine never infers one flag from another (`:focus-within`
/// is set on ancestors by whoever moves focus).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct States(u16);

impl States {
    /// No flag set.
    pub const NONE: Self = Self(0);
    /// `:hover`
    pub const HOVER: Self = Self(1);
    /// `:focus`
    pub const FOCUS: Self = Self(1 << 1);
    /// `:focus-visible`
    pub const FOCUS_VISIBLE: Self = Self(1 << 2);
    /// `:focus-within`
    pub const FOCUS_WITHIN: Self = Self(1 << 3);
    /// `:active`
    pub const ACTIVE: Self = Self(1 << 4);
    /// `:checked`
    pub const CHECKED: Self = Self(1 << 5);
    /// `:disabled`
    pub const DISABLED: Self = Self(1 << 6);
    /// `:enabled`: a form control that is not disabled. Set by the tree
    /// layer for form controls only, as the specification matches it.
    pub const ENABLED: Self = Self(1 << 7);
    /// `:invalid`: a form control whose value fails its constraints.
    pub const INVALID: Self = Self(1 << 8);

    /// Whether every flag in `other` is set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Both sets of flags.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Flags in `self` that are not in `other`.
    #[must_use]
    pub const fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }
}

pub(crate) struct NodeData {
    pub(crate) tag: String,
    pub(crate) id: Option<String>,
    pub(crate) classes: Vec<String>,
    pub(crate) attributes: BTreeMap<String, String>,
    pub(crate) states: States,
    pub(crate) parent: Option<NodeId>,
    pub(crate) children: Vec<NodeId>,
    pub(crate) previous_sibling: Option<NodeId>,
    pub(crate) next_sibling: Option<NodeId>,
    pub(crate) has_text: bool,
    pub(crate) container_size: Option<(f32, f32)>,
}

/// An element tree held in an arena. Parent, first child and both
/// sibling links are O(1); `children()` is O(n) in the child count.
pub struct Tree {
    nodes: Vec<NodeData>,
}

impl Tree {
    /// A tree with one root element of tag `tag`.
    #[must_use]
    pub fn new(tag: &str) -> Self {
        let mut tree = Self { nodes: Vec::new() };
        tree.push(tag, None);
        tree
    }

    /// The root element.
    #[must_use]
    pub fn root(&self) -> NodeId {
        NodeId(0)
    }

    /// Append a child element to `parent`.
    pub fn append(&mut self, parent: NodeId, tag: &str) -> NodeId {
        let id = self.push(tag, Some(parent));
        let previous = self.nodes[parent.0].children.last().copied();
        if let Some(prev) = previous {
            self.nodes[prev.0].next_sibling = Some(id);
        }
        self.nodes[id.0].previous_sibling = previous;
        self.nodes[parent.0].children.push(id);
        id
    }

    /// Append text to `parent`, so `:empty` no longer matches it.
    pub fn append_text(&mut self, parent: NodeId, _text: &str) {
        self.nodes[parent.0].has_text = true;
    }

    /// Set the element's id attribute.
    pub fn set_id(&mut self, node: NodeId, id: &str) {
        self.nodes[node.0].id = Some(id.to_owned());
    }

    /// Add a class.
    pub fn add_class(&mut self, node: NodeId, class: &str) {
        self.nodes[node.0].classes.push(class.to_owned());
    }

    /// Set an attribute.
    pub fn set_attribute(&mut self, node: NodeId, name: &str, value: &str) {
        self.nodes[node.0]
            .attributes
            .insert(name.to_owned(), value.to_owned());
    }

    /// Replace the element's state flags.
    pub fn set_states(&mut self, node: NodeId, states: States) {
        self.nodes[node.0].states = states;
    }

    /// Record the laid-out size `@container` queries on descendants use.
    pub fn set_container_size(&mut self, node: NodeId, width: f32, height: f32) {
        self.nodes[node.0].container_size = Some((width, height));
    }

    /// A handle that implements [`Element`](crate::selector::Element).
    #[must_use]
    pub fn node(&self, id: NodeId) -> Node<'_> {
        Node::new(self, id)
    }

    /// Every node in document order.
    pub fn ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        (0..self.nodes.len()).map(NodeId)
    }

    pub(crate) fn data(&self, id: NodeId) -> &NodeData {
        &self.nodes[id.0]
    }

    fn push(&mut self, tag: &str, parent: Option<NodeId>) -> NodeId {
        self.nodes.push(NodeData {
            tag: tag.to_owned(),
            id: None,
            classes: Vec::new(),
            attributes: BTreeMap::new(),
            states: States::NONE,
            parent,
            children: Vec::new(),
            previous_sibling: None,
            next_sibling: None,
            has_text: false,
            container_size: None,
        });
        NodeId(self.nodes.len() - 1)
    }
}

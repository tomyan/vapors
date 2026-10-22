//! A handle to one node of a [`Tree`], implementing the element trait.

use super::{NodeId, States, Tree};
use crate::selector::Element;

/// A node of a [`Tree`]: the tree reference and the node's index.
#[derive(Clone, Copy)]
pub struct Node<'a> {
    tree: &'a Tree,
    id: NodeId,
}

impl<'a> Node<'a> {
    pub(super) fn new(tree: &'a Tree, id: NodeId) -> Self {
        Self { tree, id }
    }

    /// The node's index in its tree.
    #[must_use]
    pub fn id(self) -> NodeId {
        self.id
    }
}

impl std::fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node({})", self.tree.data(self.id).tag)
    }
}

impl PartialEq for Node<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && std::ptr::eq(self.tree, other.tree)
    }
}

impl Element for Node<'_> {
    fn tag_name(&self) -> &str {
        &self.tree.data(self.id).tag
    }

    fn id(&self) -> Option<&str> {
        self.tree.data(self.id).id.as_deref()
    }

    fn has_class(&self, class: &str) -> bool {
        self.tree.data(self.id).classes.iter().any(|c| c == class)
    }

    fn class_names(&self) -> Vec<&str> {
        self.tree
            .data(self.id)
            .classes
            .iter()
            .map(String::as_str)
            .collect()
    }

    fn attribute(&self, name: &str) -> Option<&str> {
        self.tree
            .data(self.id)
            .attributes
            .get(name)
            .map(String::as_str)
    }

    fn parent(&self) -> Option<Self> {
        self.tree
            .data(self.id)
            .parent
            .map(|p| Self::new(self.tree, p))
    }

    fn previous_sibling(&self) -> Option<Self> {
        self.tree
            .data(self.id)
            .previous_sibling
            .map(|p| Self::new(self.tree, p))
    }

    fn next_sibling(&self) -> Option<Self> {
        self.tree
            .data(self.id)
            .next_sibling
            .map(|n| Self::new(self.tree, n))
    }

    fn first_child(&self) -> Option<Self> {
        self.tree
            .data(self.id)
            .children
            .first()
            .map(|c| Self::new(self.tree, *c))
    }

    fn is_empty(&self) -> bool {
        let data = self.tree.data(self.id);
        data.children.is_empty() && !data.has_text
    }

    fn states(&self) -> States {
        self.tree.data(self.id).states
    }

    fn container_size(&self) -> Option<(f32, f32)> {
        let mut node = self.parent();
        while let Some(n) = node {
            if let Some(size) = self.tree.data(n.id).container_size {
                return Some(size);
            }
            node = n.parent();
        }
        None
    }
}

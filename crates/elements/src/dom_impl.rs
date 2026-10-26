//! The generated code's calls, applied to the tree. A call with an id
//! the document does not hold panics naming the call (see `arena`). A
//! tag or event the tree does not implement yet is accepted here and
//! refused by `update` while it is in the document (see `check`).

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, Handler, NodeId, PropertyValue};

use crate::document::Document;
use crate::node::{Element, Kind, Node};

impl Dom for Document {
    fn create_element(&mut self, tag: &str) -> NodeId {
        self.arena
            .insert(Node::new(Kind::Element(Element::new(tag))))
    }

    fn add_style(&mut self, scope: &str, css: &str) {
        self.changed = true;
        self.dirty.everything();
        self.sheets.add_scoped(scope, css);
    }

    fn create_text(&mut self, text: &str) -> NodeId {
        self.arena.insert(Node::new(Kind::Text(text.to_owned())))
    }

    fn set_attribute(&mut self, node: NodeId, name: &str, value: &str) {
        self.changed = true;
        self.element_mut(node, "set_attribute")
            .attributes
            .insert(name.to_owned(), value.to_owned());
        self.mark_changed(node);
    }

    fn set_text(&mut self, node: NodeId, text: &str) {
        self.changed = true;
        if self.sheets.features.empty
            && let Some(parent) = self.parent(node)
        {
            self.dirty.subtree(parent);
        }
        match &mut self.arena.get_mut(node, "set_text").kind {
            Kind::Text(current) => text.clone_into(current),
            Kind::Element(element) => panic!(
                "set_text: {node:?} is a <{}> element, not a text node",
                element.tag
            ),
        }
    }

    fn set_property(&mut self, node: NodeId, name: &str, value: PropertyValue) {
        self.changed = true;
        self.element_mut(node, "set_property")
            .properties
            .insert(name.to_owned(), value);
    }

    fn append(&mut self, parent: NodeId, child: NodeId) {
        self.insert_before(parent, child, None);
    }

    /// Also marks what the move restyles: the child, and both parents'
    /// children where the sheets make siblings matter.
    fn insert_before(&mut self, parent: NodeId, child: NodeId, before: Option<NodeId>) {
        self.changed = true;
        if let Some(old) = self.parent(child) {
            self.mark_children_changed(old, None);
        }
        self.arena.place("insert_before", parent, child, before);
        self.mark_children_changed(parent, Some(child));
    }

    /// Detaches the node and frees it with its subtree: its id and its
    /// descendants' ids are stale from here on, and using one panics.
    fn remove(&mut self, node: NodeId) {
        assert!(
            node != self.root() && node != self.body(),
            "remove: {node:?} is the document's <{}>, which components mount under and which is never removed",
            if node == self.root() { "html" } else { "body" }
        );
        self.changed = true;
        if let Some(parent) = self.parent(node) {
            self.mark_children_changed(parent, None);
        }
        self.drop_focus_within(node);
        self.arena.detach(node);
        self.arena.free_subtree(node);
    }

    fn listen(&mut self, node: NodeId, event: &str, handler: Handler) {
        self.element_mut(node, "listen")
            .listeners
            .push((event.to_owned(), Rc::new(RefCell::new(handler))));
    }
}

impl Document {
    fn element_mut(&mut self, node: NodeId, call: &str) -> &mut Element {
        match &mut self.arena.get_mut(node, call).kind {
            Kind::Element(element) => element,
            Kind::Text(text) => {
                panic!("{call}: {node:?} is a text node ({text:?}), not an element")
            }
        }
    }
}

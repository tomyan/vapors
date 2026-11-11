//! The generated code's calls, applied to the tree. A call with an id
//! the document does not hold panics naming the call (see `arena`). A
//! tag or event the tree does not implement yet is accepted here and
//! refused by `update` while it is in the document (see `check`).

use vapors_dom::{Dom, Handler, NodeId, PropertyValue};

use crate::document::Document;
use crate::node::{Element, Kind, Node};

impl Dom for Document {
    fn create_element(&mut self, tag: &str) -> NodeId {
        self.arena
            .insert(Node::new(Kind::Element(Element::new(tag))))
    }

    fn declare_events(&mut self, names: &[&str]) {
        self.declared_events
            .extend(names.iter().map(|name| (*name).to_owned()));
    }

    fn add_style(&mut self, scope: &str, css: &str) {
        self.note_change();
        self.dirty.everything();
        self.sheets.add_scoped(scope, css);
    }

    fn create_text(&mut self, text: &str) -> NodeId {
        self.arena.insert(Node::new(Kind::Text(text.to_owned())))
    }

    /// The restyle it marks covers `:invalid` following a number
    /// field's `min`, `max` or `step` (3-E3c), which
    /// `invalid_follows_a_max_narrowed_under_the_value` pins.
    fn set_attribute(&mut self, node: NodeId, name: &str, value: &str) {
        self.note_change();
        self.element_mut(node, "set_attribute")
            .attributes
            .insert(name.to_owned(), value.to_owned());
        self.mark_changed(node);
        self.boxes.marks.mark(node);
        // a text field's editor follows its `value` attribute until the
        // field is edited, as the DOM's value does (E6a); a number
        // field's shown text also follows `step`, `min` and `max`
        self.editor_from_text(node);
    }

    fn set_text(&mut self, node: NodeId, text: &str) {
        self.note_change();
        if self.sheets.features.empty
            && let Some(parent) = self.parent(node)
        {
            self.dirty.subtree(parent);
        }
        match &mut self.arena.get_mut(node, "set_text").kind {
            Kind::Text(current) if current == text => {}
            Kind::Text(current) => {
                text.clone_into(current);
                if let Some(parent) = self.arena.get(node, "set_text").parent {
                    self.boxes.marks.mark(parent);
                }
            }
            Kind::Element(element) => panic!(
                "set_text: {node:?} is a <{}> element, not a text node",
                element.tag
            ),
        }
    }

    /// `checked` restyles for `:checked` (3-E3b); `value` for a number
    /// field's `:invalid` (3-E3c).
    fn set_property(&mut self, node: NodeId, name: &str, value: PropertyValue) {
        self.note_change();
        self.element_mut(node, "set_property")
            .properties
            .insert(name.to_owned(), value);
        self.boxes.marks.mark(node);
        match name {
            "checked" => self.mark_flag(node, vapors_css::tree::States::CHECKED),
            "value" => {
                self.mark_flag(node, vapors_css::tree::States::INVALID);
                self.editor_from_text(node);
            }
            _ => {}
        }
    }

    fn append(&mut self, parent: NodeId, child: NodeId) {
        self.insert_before(parent, child, None);
    }

    /// Also marks what the move restyles: the child, and both parents'
    /// children where the sheets make siblings matter.
    fn insert_before(&mut self, parent: NodeId, child: NodeId, before: Option<NodeId>) {
        self.note_change();
        if let Some(old) = self.parent(child) {
            self.mark_children_changed(old, None);
            self.boxes.marks.mark(old);
        }
        self.arena.place("insert_before", parent, child, before);
        self.boxes.marks.mark(parent);
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
        self.note_change();
        if let Some(parent) = self.parent(node) {
            self.mark_children_changed(parent, None);
            self.boxes.marks.mark(parent);
        }
        self.drop_focus_within(node);
        self.arena.detach(node);
        self.arena.free_subtree(node);
        // a number or text field's editing state goes with it (3-E3c, E6a)
        self.committed
            .retain(|field, _| self.arena.contains(*field));
        self.editors.retain(|field, _| self.arena.contains(*field));
    }

    fn listen(&mut self, node: NodeId, event: &str, handler: Handler) {
        self.element_mut(node, "listen")
            .listeners
            .push((event.to_owned(), std::rc::Rc::from(handler)));
    }

    fn current_text(&self, node: NodeId) -> Option<&str> {
        match &self.arena.get(node, "current_text").kind {
            Kind::Text(text) => Some(text),
            Kind::Element(_) => None,
        }
    }

    fn current_attribute(&self, node: NodeId, name: &str) -> Option<&str> {
        self.arena
            .get(node, "current_attribute")
            .element()?
            .attributes
            .get(name)
            .map(String::as_str)
    }

    /// The properties map, which the person's edits also write: a typed
    /// `value` (`text_from_editor`, `write_number_text`), a toggled
    /// `checked` (`set_checked`), a dragged range.
    fn current_property(&self, node: NodeId, name: &str) -> Option<&PropertyValue> {
        self.arena
            .get(node, "current_property")
            .element()?
            .properties
            .get(name)
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

impl vapors_dom::Listeners for Document {
    fn listeners_along(
        &self,
        target: NodeId,
        name: &str,
    ) -> Option<Vec<(NodeId, Vec<vapors_dom::Listener>)>> {
        self.contains(target)
            .then(|| self.listeners_chain(target, name))
    }
}

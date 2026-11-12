//! The generated code's calls, applied to the tree. A call with an id
//! the document does not hold panics naming the call (see `arena`). A
//! tag or event the tree does not implement yet is accepted here and
//! refused by `update` while it is in the document (see `check`).

use vapors_dom::{Dom, Handler, NodeId, PropertyValue, StyleKey};

use crate::document::Document;
use crate::node::{Element, Kind, Node};

impl Dom for Document {
    fn create_element(&mut self, tag: &str) -> NodeId {
        self.arena
            .insert(Node::new(Kind::Element(Element::new(tag))))
    }

    fn create_keyed_element(&mut self, key: &'static StyleKey) -> NodeId {
        self.arena
            .insert(Node::new(Kind::Element(Element::keyed(key))))
    }

    fn toggle_classes(&mut self, node: NodeId, toggles: &[(&str, bool)]) {
        if let Some(class) = self.toggled_class(node, toggles) {
            self.write_attribute(node, "class", &class);
        }
    }

    /// Every mount calls it; only an event not declared before can
    /// change an element's refusal, so only that checks every element.
    fn declare_events(&mut self, names: &[&str]) {
        let before = self.declared_events.len();
        self.declared_events
            .extend(names.iter().map(|name| (*name).to_owned()));
        if self.declared_events.len() > before {
            self.checks.all = true;
        }
    }

    /// Every mount calls it; a sheet already added changes nothing. A
    /// new one restyles every element: which elements its rules reach
    /// is not worked out.
    fn add_style(&mut self, scope: &str, css: &str) {
        if self.sheets.add_scoped(scope, css) {
            self.note_change();
            self.dirty.everything();
        }
    }

    fn create_text(&mut self, text: &str) -> NodeId {
        self.arena.insert(Node::new(Kind::Text(text.to_owned())))
    }

    /// The restyle it marks covers `:invalid` following a number
    /// field's `min`, `max` or `step` (3-E3c), which
    /// `invalid_follows_a_max_narrowed_under_the_value` pins.
    /// Refused for an attribute the element's key fixes (`dom_keyed`).
    fn set_attribute(&mut self, node: NodeId, name: &str, value: &str) {
        self.refuse_keyed_attribute(node, name);
        self.write_attribute(node, name, value);
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
            self.mark_children_changed(old, child, false);
            self.boxes.marks.mark_children(old);
        }
        self.arena.place("insert_before", parent, child, before);
        self.checks.added.push(child);
        self.boxes.marks.mark_children(parent);
        self.mark_children_changed(parent, child, true);
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
            self.mark_children_changed(parent, node, false);
            self.boxes.marks.mark_children(parent);
        }
        self.drop_focus_within(node);
        self.forget_subtree(node);
        self.arena.detach(node);
        self.arena.free_subtree(node);
        // a number or text field's editing state goes with it (3-E3c, E6a)
        self.committed
            .retain(|field, _| self.arena.contains(*field));
        self.editors.retain(|field, _| self.arena.contains(*field));
    }

    fn listen(&mut self, node: NodeId, event: &str, handler: Handler) {
        self.checks.marked.insert(node);
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
    /// Writes the attribute and marks what follows it.
    fn write_attribute(&mut self, node: NodeId, name: &str, value: &str) {
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

    pub(crate) fn element_mut(&mut self, node: NodeId, call: &str) -> &mut Element {
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

//! Dispatching events into the recorded tree: target first, then up
//! through the ancestors, until a listener stops propagation. The dom
//! is not borrowed while a listener runs, so listeners may use it.

use std::cell::RefCell;
use std::rc::Rc;

use super::{Kind, RecordingDom};
use crate::{Event, NodeId, PropertyValue};

impl RecordingDom {
    /// A click on `target`.
    pub fn click(dom: &Rc<RefCell<Self>>, target: NodeId) {
        Self::dispatch(dom, target, Event::new("click"));
    }

    /// Typing into `target`: sets its `value` property and fires `input`.
    pub fn input(dom: &Rc<RefCell<Self>>, target: NodeId, value: &str) {
        dom.borrow_mut()
            .set_property_quietly(target, "value", PropertyValue::Text(value.into()));
        Self::dispatch(dom, target, Event::new("input").with_value(value));
    }

    /// Toggling `target`: sets its `checked` property and fires `change`.
    pub fn change(dom: &Rc<RefCell<Self>>, target: NodeId, checked: bool) {
        dom.borrow_mut()
            .set_property_quietly(target, "checked", PropertyValue::Bool(checked));
        let value = dom.borrow().text_property(target, "value");
        let event = Event::new("change")
            .with_checked(checked)
            .with_value(&value);
        Self::dispatch(dom, target, event);
    }

    /// Delivers `event` to `target` and its ancestors.
    pub fn dispatch(dom: &Rc<RefCell<Self>>, target: NodeId, mut event: Event) {
        let path = dom.borrow().path_to_root(target);
        for node in path {
            let listeners = dom
                .borrow()
                .listeners
                .get(&(node, event.name().to_string()))
                .cloned()
                .unwrap_or_default();
            for listener in listeners {
                (listener.borrow_mut())(&mut event);
            }
            if event.propagation_stopped() {
                return;
            }
        }
    }

    fn path_to_root(&self, node: NodeId) -> Vec<NodeId> {
        let mut path = vec![node];
        let mut current = node;
        while let Some(parent) = self.record(current).parent {
            path.push(parent);
            current = parent;
        }
        path
    }

    /// Sets a property as the user would, without recording a call.
    fn set_property_quietly(&mut self, node: NodeId, name: &str, value: PropertyValue) {
        if let Kind::Element { properties, .. } = &mut self.record_mut(node).kind {
            properties.insert(name.into(), value);
        }
    }

    fn text_property(&self, node: NodeId, name: &str) -> String {
        match self.property(node, name) {
            Some(PropertyValue::Text(text)) => text,
            _ => String::new(),
        }
    }
}

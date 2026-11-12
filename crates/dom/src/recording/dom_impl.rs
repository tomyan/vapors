//! The `Dom` implementation: each call is recorded and applied to the tree.

use std::collections::BTreeMap;
use std::rc::Rc;

use super::{Call, Kind, RecordingDom};
use crate::{Dom, Handler, NodeId, PropertyValue, StyleKey};

impl Dom for RecordingDom {
    fn create_element(&mut self, tag: &str) -> NodeId {
        let node = self.push(Kind::Element {
            tag: tag.into(),
            attributes: BTreeMap::new(),
            properties: BTreeMap::new(),
            key: None,
        });
        self.calls.push(Call::CreateElement {
            node,
            tag: tag.into(),
        });
        node
    }

    fn create_keyed_element(&mut self, key: &'static StyleKey) -> NodeId {
        let node = self.push(Self::keyed(key));
        self.calls.push(Call::CreateKeyedElement { node, key });
        if let Some(refusal) = key.shape_refusal() {
            self.refuse(node, format!("create_keyed_element: {refusal}"));
        }
        node
    }

    fn toggle_classes(&mut self, node: NodeId, toggles: &[(&str, bool)]) {
        match self.toggle(node, toggles) {
            Ok(()) => self.calls.push(Call::ToggleClasses {
                node,
                toggles: (toggles.iter())
                    .map(|(name, on)| ((*name).to_owned(), *on))
                    .collect(),
            }),
            Err(refusal) => self.refuse(node, format!("toggle_classes: {refusal}")),
        }
    }

    fn style_key(&self, node: NodeId) -> Option<&'static StyleKey> {
        match &self.record(node).kind {
            Kind::Element { key, .. } => key.map(|(key, _)| key),
            Kind::Text(_) => None,
        }
    }

    fn create_text(&mut self, text: &str) -> NodeId {
        let node = self.push(Kind::Text(text.into()));
        self.calls.push(Call::CreateText {
            node,
            text: text.into(),
        });
        node
    }

    /// Refuses an attribute the element's key fixes (`StyleKey::refusal`).
    fn set_attribute(&mut self, node: NodeId, name: &str, value: &str) {
        if let Some(refusal) = self.style_key(node).and_then(|key| key.refusal(name)) {
            return self.refuse(node, format!("set_attribute: {refusal}"));
        }
        if let Kind::Element { attributes, .. } = &mut self.record_mut(node).kind {
            attributes.insert(name.into(), value.into());
        }
        self.calls.push(Call::SetAttribute {
            node,
            name: name.into(),
            value: value.into(),
        });
    }

    fn set_text(&mut self, node: NodeId, text: &str) {
        self.record_mut(node).kind = Kind::Text(text.into());
        self.calls.push(Call::SetText {
            node,
            text: text.into(),
        });
    }

    fn set_property(&mut self, node: NodeId, name: &str, value: PropertyValue) {
        if let Kind::Element { properties, .. } = &mut self.record_mut(node).kind {
            properties.insert(name.into(), value.clone());
        }
        self.calls.push(Call::SetProperty {
            node,
            name: name.into(),
            value,
        });
    }

    fn append(&mut self, parent: NodeId, child: NodeId) {
        self.place(parent, child, None);
        self.calls.push(Call::Append { parent, child });
    }

    fn insert_before(&mut self, parent: NodeId, child: NodeId, before: Option<NodeId>) {
        self.place(parent, child, before);
        self.calls.push(Call::InsertBefore {
            parent,
            child,
            before,
        });
    }

    fn remove(&mut self, node: NodeId) {
        self.detach(node);
        self.mark_removed(node);
        self.calls.push(Call::Remove { node });
    }

    fn listen(&mut self, node: NodeId, event: &str, handler: Handler) {
        self.listeners
            .entry((node, event.into()))
            .or_default()
            .push(Rc::from(handler));
        self.calls.push(Call::Listen {
            node,
            event: event.into(),
        });
    }

    fn declare_events(&mut self, names: &[&str]) {
        self.calls.push(Call::DeclareEvents(
            names.iter().map(|name| (*name).to_owned()).collect(),
        ));
    }

    fn add_style(&mut self, scope: &str, css: &str) {
        self.calls.push(Call::AddStyle {
            scope: scope.into(),
            css: css.into(),
        });
    }

    fn current_text(&self, node: NodeId) -> Option<&str> {
        match &self.record(node).kind {
            Kind::Text(text) => Some(text),
            Kind::Element { .. } => None,
        }
    }

    fn current_attribute(&self, node: NodeId, name: &str) -> Option<&str> {
        match &self.record(node).kind {
            Kind::Element { attributes, .. } => attributes.get(name).map(String::as_str),
            Kind::Text(_) => None,
        }
    }

    fn current_property(&self, node: NodeId, name: &str) -> Option<&PropertyValue> {
        match &self.record(node).kind {
            Kind::Element { properties, .. } => properties.get(name),
            Kind::Text(_) => None,
        }
    }
}

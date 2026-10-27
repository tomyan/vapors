//! A checkbox (3-E3b): `input type=checkbox`, checked by its `checked`
//! property once set (by a listener's signal or by the user), else by
//! its `checked` attribute. A click flips it before the click's
//! listeners run, as HTML's legacy pre-activation behaviour has it (so
//! a listener reads the new state), flips it back if one prevents the
//! click, and otherwise fires `input` and `change` carrying `checked`,
//! as Chromium orders them; Space clicks it on keyup, as a button.
//! `:checked` follows the state.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_css::tree::States;
use vapors_dom::{Event, NodeId, PropertyValue};

use crate::document::Document;
use crate::events::tree::dispatch_bubbling;
use crate::node::Element;

/// The content size a checkbox is laid out at without one of its own,
/// CSS px: Chromium's 13 px border box (`fixtures/checkbox.wisp`) less
/// the user-agent sheet's 1 px border, which `tests/checkbox_layout.rs`
/// holds to 13 together. As a flex item it stretches across the line,
/// as Chromium's does.
pub(crate) const NATURAL_SIZE: [f32; 2] = [11.0, 11.0];

/// Whether `element` is a checkbox that is checked, or `None` for
/// another element.
pub(crate) fn checked_of(element: &Element) -> Option<bool> {
    if element.tag != "input" || !crate::check::is_type(element, "checkbox") {
        return None;
    }
    Some(match element.properties.get("checked") {
        Some(PropertyValue::Bool(checked)) => *checked,
        // a property of another shape is refused by `check`
        Some(PropertyValue::Text(_) | PropertyValue::Number(_)) | None => {
            element.attributes.contains_key("checked")
        }
    })
}

impl Document {
    /// Whether a checkbox is checked; `None` for an element that is not
    /// one.
    #[must_use]
    pub fn checked(&self, node: NodeId) -> Option<bool> {
        self.arena
            .get(node, "checked")
            .element()
            .and_then(checked_of)
    }

    /// Sets a checkbox's state, restyling for `:checked`; whether it
    /// changed.
    pub(crate) fn set_checked(&mut self, node: NodeId, checked: bool) -> bool {
        if self.checked(node) == Some(checked) {
            return false;
        }
        if let Some(element) = self.arena.get_mut(node, "checked").element_mut() {
            element
                .properties
                .insert("checked".to_owned(), PropertyValue::Bool(checked));
        }
        self.changed = true;
        self.mark_flag(node, States::CHECKED);
        true
    }

    /// A checkbox the user can toggle: one that is not disabled.
    fn live_checkbox(&self, node: NodeId) -> bool {
        self.checked(node).is_some() && !self.is_disabled_control(node)
    }
}

/// A click's pre-activation on `target`: a live checkbox flips, and
/// the state it had is returned for `after_click`; `None` for another
/// element.
pub(crate) fn before_click(document: &Rc<RefCell<Document>>, target: NodeId) -> Option<bool> {
    let mut doc = document.borrow_mut();
    if !doc.live_checkbox(target) {
        return None;
    }
    let was = doc.checked(target)?;
    doc.set_checked(target, !was);
    Some(was)
}

/// A click's activation on `target`, which `before_click` flipped from
/// `was`: flipped back when a listener prevented the click, else
/// `input` then `change` with the new state.
pub(crate) fn after_click(
    document: &Rc<RefCell<Document>>,
    target: NodeId,
    was: bool,
    prevented: bool,
) {
    if !document.borrow().contains(target) {
        return;
    }
    if prevented {
        document.borrow_mut().set_checked(target, was);
        return;
    }
    for name in ["input", "change"] {
        dispatch_bubbling(document, target, Event::new(name).with_checked(!was));
    }
}

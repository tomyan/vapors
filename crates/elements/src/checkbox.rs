//! A checkbox (3-E3b): `input type=checkbox`, checked by its `checked`
//! property once set (by a listener's signal or by the user), else by
//! its `checked` attribute. A click toggles it after the click's
//! listeners ran, unless one prevented it, then `input` and `change`
//! fire carrying `checked`, as Chromium orders them; Space clicks it on
//! keyup, as a button. `:checked` follows the state.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_css::tree::States;
use vapors_dom::{Event, NodeId, PropertyValue};

use crate::document::Document;
use crate::events::tree::{deliver, dispatch_bubbling};
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

/// `click` on `target`, bubbling, then its default action: a live
/// checkbox toggles and fires `input` then `change`, unless a listener
/// prevented the click.
pub(crate) fn click(document: &Rc<RefCell<Document>>, target: NodeId, event: Event) {
    let (event, _) = deliver(document, target, event, true);
    // a listener may have removed the target (a toast's dismiss button)
    let live = document.borrow().contains(target) && document.borrow().live_checkbox(target);
    if event.default_prevented() || !live {
        return;
    }
    let was = document.borrow().checked(target).unwrap_or(false);
    document.borrow_mut().set_checked(target, !was);
    for name in ["input", "change"] {
        dispatch_bubbling(document, target, Event::new(name).with_checked(!was));
    }
}

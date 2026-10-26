//! Keys (3-E2a): `keydown` and `keyup` go to the focused element, else
//! the body. Unless a listener prevents it, Tab and Shift+Tab move focus
//! (on keydown), Enter clicks a focused button on keydown and Space on
//! keyup, as browsers do.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Event, NodeId};

use crate::document::Document;
use crate::events::dispatch_bubbling;

/// The modifier keys held with a key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // the DOM's shiftKey, ctrlKey, altKey and metaKey, one each
pub struct Modifiers {
    /// Shift.
    pub shift: bool,
    /// Control.
    pub control: bool,
    /// Alt (Option on macOS).
    pub alt: bool,
    /// Meta: Command on macOS, the Windows or Super key elsewhere.
    pub meta: bool,
}

impl Document {
    /// `key` went down: `keydown`, then its default action.
    pub fn key_pressed(document: &Rc<RefCell<Document>>, key: &str, modifiers: Modifiers) {
        let target = document.borrow().key_target();
        if dispatch_bubbling(document, target, Event::new("keydown").with_key(key)) {
            return;
        }
        match key {
            // Control, Alt or Meta with Tab is the browser's or the window
            // manager's, which Chromium does not navigate on
            "Tab" if !(modifiers.control || modifiers.alt || modifiers.meta) => {
                let next = document.borrow().next_in_tab_order(modifiers.shift);
                Self::focus(document, next);
            }
            "Enter" => Self::activate(document, target),
            " " => document.borrow_mut().space_on = Some(target),
            _ => {}
        }
    }

    /// `key` came up: `keyup`, then Space's click on the element it went
    /// down on.
    pub fn key_released(document: &Rc<RefCell<Document>>, key: &str, _modifiers: Modifiers) {
        let target = document.borrow().key_target();
        // only Space's own keyup takes it: a key tapped while Space is
        // held does not end the press
        let space_on = if key == " " {
            document.borrow_mut().space_on.take()
        } else {
            None
        };
        if dispatch_bubbling(document, target, Event::new("keyup").with_key(key)) {
            return;
        }
        if key == " " && space_on == Some(target) {
            Self::activate(document, target);
        }
    }

    /// The focused element, else the body.
    fn key_target(&self) -> NodeId {
        self.active_element().unwrap_or_else(|| self.body())
    }

    /// A focused button's click, unless it is disabled.
    fn activate(document: &Rc<RefCell<Document>>, target: NodeId) {
        let button = document.borrow().tag(target) == Some("button")
            && !document.borrow().in_disabled_button(target);
        if button {
            dispatch_bubbling(document, target, Event::new("click"));
        }
    }
}

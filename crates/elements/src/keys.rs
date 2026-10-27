//! Keys (3-E2a): `keydown` and `keyup` go to the focused element, else
//! the body. Unless a listener prevents it, Tab and Shift+Tab move focus
//! (on keydown), Enter clicks a focused button on keydown and Space on
//! keyup, as browsers do, and the arrow, Page, Home and End keys step a
//! focused range (3-E3a, `range_input`).

use std::cell::RefCell;
use std::rc::Rc;

use vapors_css::tree::States;
use vapors_dom::{Event, NodeId};

use crate::document::Document;
use crate::events::tree::dispatch_bubbling;

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
        document.borrow_mut().keyboard_interaction(key);
        let target = document.borrow().key_target();
        if dispatch_bubbling(document, target, Event::new("keydown").with_key(key)) {
            return;
        }
        Self::key_default(document, target, key, modifiers);
    }

    /// A key other than a modifier alone is a keyboard interaction for
    /// `:focus-visible`, as in Chromium (review-v82): focus arriving
    /// next shows the ring, and so does the focused element now, as
    /// Chromium does when a key is pressed after a press focused it
    /// (3-E3a's oracle).
    fn keyboard_interaction(&mut self, key: &str) {
        if matches!(key, "Shift" | "Control" | "Alt" | "Meta") {
            return;
        }
        self.keyboard = true;
        let active = self.active_element();
        self.set_flag_on(States::FOCUS_VISIBLE, active);
    }

    /// A key's default action on `target`, after an unprevented keydown.
    fn key_default(
        document: &Rc<RefCell<Document>>,
        target: NodeId,
        key: &str,
        modifiers: Modifiers,
    ) {
        if matches!(key, "Backspace" | "Enter" | "ArrowUp" | "ArrowDown")
            && document.borrow().number_field(target).is_some()
        {
            crate::number_input::key(document, target, key, modifiers);
        }
        match key {
            // Control, Alt or Meta with Tab is the browser's or the window
            // manager's, which Chromium does not navigate on
            "Tab" if !(modifiers.control || modifiers.alt || modifiers.meta) => {
                let next = document.borrow().next_in_tab_order(modifiers.shift);
                Self::focus(document, next);
            }
            "Enter" => Self::activate(document, target, &["button"]),
            " " => document.borrow_mut().space_on = Some(target),
            "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" | "PageUp" | "PageDown"
            | "Home" | "End" => {
                crate::range_input::key(document, target, key, modifiers);
            }
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
            Self::activate(document, target, &["button", "checkbox"]);
        }
    }

    /// The focused element, else the body.
    fn key_target(&self) -> NodeId {
        self.active_element().unwrap_or_else(|| self.body())
    }

    /// A focused control's click, when it is one of `kinds` (`button`,
    /// or `checkbox` for Space, which Enter does not click, as in
    /// Chromium) and not disabled; a checkbox then toggles.
    fn activate(document: &Rc<RefCell<Document>>, target: NodeId, kinds: &[&str]) {
        let doc = document.borrow();
        let kind = match doc.tag(target) {
            Some("button") => "button",
            Some("input") if doc.checked(target).is_some() => "checkbox",
            _ => return,
        };
        if !kinds.contains(&kind) || doc.in_disabled_button(target) {
            return;
        }
        drop(doc);
        crate::events::tree::dispatch_click(document, target, Event::new("click"));
    }
}

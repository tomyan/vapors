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
use crate::focus_style::Focus;

pub use vapors_dom::Modifiers;

/// A key as the platform reports it (`vapors_platform::Event::Key`):
/// the DOM's `key`, its `code` (the key's place on the keyboard, the
/// same for its down and up), whether it is a held key's auto-repeat,
/// and the modifiers held. Its `keydown` and `keyup` carry all four.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Keystroke<'a> {
    /// The key as the DOM names it (`"a"`, `"Enter"`, `" "`).
    pub key: &'a str,
    /// The DOM's `code` (`"KeyA"`, `"ShiftLeft"`); empty when unknown.
    pub code: &'a str,
    /// A held key's auto-repeat.
    pub repeat: bool,
    /// The modifier keys held.
    pub modifiers: Modifiers,
}

/// One stage of a key's default action: whether it took the key.
type DefaultAction = fn(&Rc<RefCell<Document>>, NodeId, &str, Modifiers) -> bool;

impl Keystroke<'_> {
    /// Its `keydown` or `keyup`.
    fn event(self, name: &str) -> Event {
        Event::new(name)
            .with_key(self.key)
            .with_code(self.code)
            .with_repeat(self.repeat)
            .with_modifiers(self.modifiers)
    }
}

/// Whether `key` types text into a field: a single printable character
/// with neither Control nor Meta (its text then arrives as a character).
/// Named keys (F1 to F12, Escape, Tab, `PageUp`) do not, so a field leaves
/// them to the app.
pub(crate) fn types_text(key: &str, modifiers: Modifiers) -> bool {
    let mut chars = key.chars();
    let single = matches!((chars.next(), chars.next()), (Some(c), None) if !c.is_control());
    single && !modifiers.control && !modifiers.meta
}

impl Document {
    /// `key` went down with no `code` and not repeating: see
    /// [`Document::key_pressed_with`].
    pub fn key_pressed(document: &Rc<RefCell<Document>>, key: &str, modifiers: Modifiers) -> bool {
        let press = Keystroke {
            key,
            modifiers,
            ..Keystroke::default()
        };
        Self::key_pressed_with(document, press)
    }

    /// A key went down: `keydown`, then its default action. Whether the
    /// document used it: a listener prevented its default, or a control
    /// took it (focus moved, a button pressed, a field or range changed).
    /// A key it did not use is the app's, as a browser gives the page's
    /// shortcuts the keys no control takes. A focused element hidden
    /// since the last settle blurs first (`focus_fixup`).
    pub fn key_pressed_with(document: &Rc<RefCell<Document>>, press: Keystroke<'_>) -> bool {
        Self::blur_unfocusable(document);
        document.borrow_mut().keyboard_interaction(press.key);
        let target = document.borrow().key_target();
        if dispatch_bubbling(document, target, press.event("keydown")) {
            return true;
        }
        Self::key_default(document, target, press.key, press.modifiers)
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

    /// A key's default action on `target`, after an unprevented keydown:
    /// a number field's, a text field's, then the page's; whether a
    /// control took it. Each one's listeners may remove the target, so
    /// each stage re-resolves it and the rest stop (`still_in`).
    fn key_default(
        document: &Rc<RefCell<Document>>,
        target: NodeId,
        key: &str,
        modifiers: Modifiers,
    ) -> bool {
        let stages: [DefaultAction; 3] = [
            crate::number_input::key,
            crate::text_input::key,
            Self::page_default,
        ];
        let mut used = false;
        for stage in stages {
            let Some(target) = document.borrow().still_in(target) else {
                break;
            };
            used |= stage(document, target, key, modifiers);
        }
        used
    }

    /// Tab's focus move, Enter's and Space's press, and a range's step;
    /// whether one happened.
    fn page_default(
        document: &Rc<RefCell<Document>>,
        target: NodeId,
        key: &str,
        modifiers: Modifiers,
    ) -> bool {
        match key {
            // Control, Alt or Meta with Tab is the browser's or the window
            // manager's, which Chromium does not navigate on
            "Tab" if !(modifiers.control || modifiers.alt || modifiers.meta) => {
                let next = document.borrow().next_in_tab_order(modifiers.shift);
                let went = Self::focus(document, next);
                // the Tab order chose `next` by the rule `focus` applies
                assert_eq!(
                    went,
                    Focus::Done,
                    "Tab: {next:?} was chosen by the Tab order as focusable, but focus answered {went:?}; next_in_tab_order and focusability disagree (focus.rs); reproduce with the page in tests/tab_focusability.rs"
                );
                true
            }
            "Enter" => Self::activate(document, target, &["button"]),
            " " => {
                document.borrow_mut().space_on = Some(target);
                document
                    .borrow()
                    .activatable(target, &["button", "checkbox"])
            }
            "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" | "PageUp" | "PageDown"
            | "Home" | "End" => crate::range_input::key(document, target, key, modifiers),
            _ => false,
        }
    }

    /// `key` came up with no `code`: see [`Document::key_released_with`].
    pub fn key_released(document: &Rc<RefCell<Document>>, key: &str, modifiers: Modifiers) {
        let press = Keystroke {
            key,
            modifiers,
            ..Keystroke::default()
        };
        Self::key_released_with(document, press);
    }

    /// A key came up: `keyup`, then Space's click on the element it went
    /// down on.
    pub fn key_released_with(document: &Rc<RefCell<Document>>, press: Keystroke<'_>) {
        Self::blur_unfocusable(document);
        let target = document.borrow().key_target();
        // only Space's own keyup takes it: a key tapped while Space is
        // held does not end the press
        let space_on = if press.key == " " {
            document.borrow_mut().space_on.take()
        } else {
            None
        };
        if dispatch_bubbling(document, target, press.event("keyup")) {
            return;
        }
        let still = document.borrow().still_in(target);
        if press.key == " " && space_on == still {
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
    fn activate(document: &Rc<RefCell<Document>>, target: NodeId, kinds: &[&str]) -> bool {
        if !document.borrow().activatable(target, kinds) {
            return false;
        }
        crate::events::tree::dispatch_click(document, target, Event::new("click"));
        true
    }

    /// Whether `target` is one of `kinds` (`button`, `checkbox`) and not
    /// disabled: a key would click it.
    fn activatable(&self, target: NodeId, kinds: &[&str]) -> bool {
        let kind = match self.tag(target) {
            Some("button") => "button",
            Some("input") if self.checked(target).is_some() => "checkbox",
            _ => return false,
        };
        kinds.contains(&kind) && !self.in_disabled_button(target)
    }
}

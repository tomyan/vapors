//! A number field's editing (3-E3c), the least that takes a typed
//! exact value before E6's editor: the caret is always at the end;
//! typed characters a number takes (`number::typeable`) go there, or
//! replace the whole text when it is selected (focus by keyboard
//! selects it, as Chromium does); Backspace deletes the last one.
//! `input` fires with each edit, carrying the DOM's sanitized `value`.
//! Enter and blur commit (`number::Field::committed`), arrows step by
//! `step` (Shift by ten, vapors' own), each with `input` and `change`.
//! The last committed number is kept per field, so `change` fires
//! only when the commit differs from it, as Chromium fires it.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Event, NodeId, PropertyValue};

use crate::document::Document;
use crate::events::tree::dispatch_bubbling;
use crate::keys::Modifiers;
use crate::number::{Field, typeable};
use crate::range::written;

/// Chromium's steps for Shift with an arrow key, vapors' own default
/// action (`2026-10-27-direct-tools-first`, 3).
const COARSE_STEPS: f64 = 10.0;

impl Document {
    /// The field `node` is, when it is an `input type=number`.
    pub(crate) fn number_field(&self, node: NodeId) -> Option<Field> {
        let element = self.arena.get(node, "number").element()?;
        if element.tag != "input" || !crate::check::is_type(element, "number") {
            return None;
        }
        Some(Field::of(&element.attributes, &element.properties))
    }

    /// Whether the field's text is selected whole, so typing replaces it.
    #[must_use]
    pub fn text_selected(&self, node: NodeId) -> bool {
        self.selected_all == Some(node)
    }

    /// The number the field last committed, from the field's text when
    /// it has not yet.
    fn last_committed(&self, node: NodeId, field: &Field) -> f64 {
        self.committed
            .get(&node)
            .copied()
            .or_else(|| field.committed())
            .unwrap_or_else(|| field.clamped(0.0))
    }

    /// A number field the user can edit: focused and not disabled.
    fn editable_number(&self, node: NodeId) -> Option<Field> {
        if self.is_disabled_control(node) {
            return None;
        }
        self.number_field(node)
    }

    /// Sets the field's text, clearing the selection; whether it changed.
    pub(crate) fn set_number_text(&mut self, node: NodeId, text: String) -> bool {
        let same = self.number_field(node).is_some_and(|f| f.text == text);
        self.selected_all = None;
        if same {
            return false;
        }
        self.changed = true;
        if let Some(element) = self.arena.get_mut(node, "number").element_mut() {
            element
                .properties
                .insert("value".to_owned(), PropertyValue::Text(text));
        }
        // the text decides `:invalid`
        self.mark_flag(node, vapors_css::tree::States::INVALID);
        true
    }

    /// Focus arrived at `node` by `keyboard` or not: a number field
    /// remembers what it would commit against, and keyboard focus
    /// selects its text whole.
    pub(crate) fn number_focused(&mut self, node: NodeId, keyboard: bool) {
        let Some(field) = self.number_field(node) else {
            return;
        };
        let committed = self.last_committed(node, &field);
        self.committed.insert(node, committed);
        if keyboard {
            self.selected_all = Some(node);
            self.changed = true;
        }
    }
}

/// `text` was typed into `node`, a focused number field: the characters
/// a number takes go to the end, or replace a selected text.
pub(crate) fn typed(document: &Rc<RefCell<Document>>, node: NodeId, text: &str) {
    let Some(field) = document.borrow().editable_number(node) else {
        return;
    };
    let accepted: String = text.chars().filter(|c| typeable(*c)).collect();
    if accepted.is_empty() {
        return;
    }
    let base = if document.borrow().text_selected(node) {
        String::new()
    } else {
        field.text
    };
    if document
        .borrow_mut()
        .set_number_text(node, base + &accepted)
    {
        fire(document, node, "input");
    }
}

impl Document {
    /// `text` was typed (the platform's `Character`): it goes to the
    /// focused number field, control characters left out.
    pub fn character_typed(document: &Rc<RefCell<Document>>, text: &str) {
        let printable: String = text.chars().filter(|c| !c.is_control()).collect();
        let Some(node) = document.borrow().active_element() else {
            return;
        };
        if !printable.is_empty() {
            typed(document, node, &printable);
        }
    }
}

/// `input` on `node` after a scrub's step.
pub(crate) fn fire_input(document: &Rc<RefCell<Document>>, node: NodeId) {
    fire(document, node, "input");
}

/// `key` went down on `node`, a focused number field: Backspace,
/// Enter and the arrows act; other keys are typed as characters.
pub(crate) fn key(document: &Rc<RefCell<Document>>, node: NodeId, key: &str, modifiers: Modifiers) {
    let Some(field) = document.borrow().editable_number(node) else {
        return;
    };
    match key {
        "Backspace" => backspace(document, node, field),
        "Enter" => commit(document, node),
        "ArrowUp" | "ArrowDown" => {
            let sign = if key == "ArrowUp" { 1.0 } else { -1.0 };
            let steps = if modifiers.shift { COARSE_STEPS } else { 1.0 };
            step(document, node, &field, sign * steps);
        }
        _ => {}
    }
}

fn backspace(document: &Rc<RefCell<Document>>, node: NodeId, field: Field) {
    let mut text = field.text;
    if document.borrow().text_selected(node) {
        text.clear();
    } else {
        text.pop();
    }
    if document.borrow_mut().set_number_text(node, text) {
        fire(document, node, "input");
    }
}

fn step(document: &Rc<RefCell<Document>>, node: NodeId, field: &Field, steps: f64) {
    let fallback = document.borrow().last_committed(node, field);
    let value = field.stepped(steps, fallback);
    if document.borrow_mut().set_number_text(node, written(value)) {
        fire(document, node, "input");
    }
    settle(document, node, value);
}

/// Commits `node`'s text: a number clamped into the range, or the last
/// committed number when the text is no number; `change` when it
/// differs from the last committed.
pub(crate) fn commit(document: &Rc<RefCell<Document>>, node: NodeId) {
    let Some(field) = document.borrow().number_field(node) else {
        return;
    };
    let last = document.borrow().last_committed(node, &field);
    let value = field.committed().unwrap_or(last);
    document.borrow_mut().set_number_text(node, written(value));
    settle(document, node, value);
}

/// `value` becomes the last committed, with `change` when it is new.
fn settle(document: &Rc<RefCell<Document>>, node: NodeId, value: f64) {
    let last = document.borrow_mut().committed.insert(node, value);
    if last != Some(value) {
        fire(document, node, "change");
    }
}

/// `input` or `change` on `node`, carrying the DOM's value, and the
/// number when the text is one (an `input` mid-typing may carry none;
/// a `change` always does).
fn fire(document: &Rc<RefCell<Document>>, node: NodeId, name: &str) {
    let Some(field) = document.borrow().number_field(node) else {
        return;
    };
    let mut event = Event::new(name).with_value(&field.value());
    if let Some(number) = field.number() {
        event = event.with_value_number(number);
    }
    dispatch_bubbling(document, node, event);
}

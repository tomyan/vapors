//! A number field's editing (3-E3c): its caret and selection are a
//! text field's (`text_field::Editor`, over what the field shows,
//! `number_shown.rs`), moved by the arrows, Home and End and the
//! pointer as a text field's are (`text_input`, `text_pointer`);
//! typed characters a number takes (`number::typeable`) replace the
//! selection, and Backspace and Delete delete as a text field's do.
//! Focus by keyboard selects the text whole, other focus puts the
//! caret at the end, as Chromium does.
//! `input` fires with each edit, carrying the DOM's sanitized `value`.
//! Enter and blur commit (`number::Field::committed`), arrows step by
//! `step` (Shift by ten, vapors' own), each with `input` and `change`.
//! The last committed number is kept per field, so `change` fires
//! only when the commit differs from it, as Chromium fires it.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Event, NodeId};

use crate::document::Document;
use crate::events::tree::dispatch_bubbling;
use crate::keys::Modifiers;
use crate::number::{Field, typeable};
use crate::range::written;
use crate::text_history::EditKind;

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
        self.editors.get(&node).is_some_and(|editor| {
            let range = editor.plain.raw_selection().text_range();
            !range.is_empty() && range == (0..editor.plain.raw_text().len())
        })
    }

    /// The number a step or scrub counts from: the text's, else the
    /// last committed.
    pub(crate) fn number_base(&self, node: NodeId, field: &Field) -> f64 {
        field
            .number()
            .unwrap_or_else(|| self.last_committed(node, field))
    }

    /// A number field the user can edit: focused and not disabled.
    fn editable_number(&self, node: NodeId) -> Option<Field> {
        if self.is_disabled_control(node) {
            return None;
        }
        self.number_field(node)
    }
}

/// `text` was typed into `node`, a focused number field: the characters
/// a number takes replace the selection.
pub(crate) fn typed(document: &Rc<RefCell<Document>>, node: NodeId, text: &str) {
    if document.borrow().editable_number(node).is_none() {
        return;
    }
    let accepted: String = text.chars().filter(|c| typeable(*c)).collect();
    if accepted.is_empty() {
        return;
    }
    document.borrow_mut().edit(node, EditKind::Insert, |d| {
        d.insert_or_replace_selection(&accepted);
    });
    edited(document, node);
}

/// After an edit: the field's `value` takes the editor's text and
/// `input` fires.
fn edited(document: &Rc<RefCell<Document>>, node: NodeId) {
    let text = document.borrow_mut().editor(node).plain.text().to_string();
    if document.borrow_mut().write_number_text(node, text) {
        fire(document, node, "input");
    }
}

/// `input` on `node` after a scrub's step.
pub(crate) fn fire_input(document: &Rc<RefCell<Document>>, node: NodeId) {
    fire(document, node, "input");
}

/// `key` went down on `node`, a focused number field: Backspace,
/// Delete, Enter, the arrows, Home, End and select-all act; other keys
/// are typed.
pub(crate) fn key(
    document: &Rc<RefCell<Document>>,
    node: NodeId,
    key: &str,
    modifiers: Modifiers,
) -> bool {
    let Some(field) = document.borrow().editable_number(node) else {
        return false;
    };
    match key {
        "Backspace" | "Delete" => {
            let word = modifiers.control;
            crate::text_delete::delete_key(document, node, key == "Backspace", word);
            edited(document, node);
        }
        "Enter" => commit(document, node),
        _ if crate::shortcuts::selected_all(document, node, key, modifiers) => {}
        "ArrowUp" | "ArrowDown" => {
            let sign = if key == "ArrowUp" { 1.0 } else { -1.0 };
            let steps = if modifiers.shift { COARSE_STEPS } else { 1.0 };
            step(document, node, &field, sign * steps);
        }
        // the field's typing (digits, a sign, a point, an exponent) arrives
        // as a character; a stray letter is still the field's, not a shortcut
        _ => {
            let (shift, word) = (modifiers.shift, modifiers.control);
            return crate::text_input::moved(document, node, key, shift, word)
                || crate::keys::types_text(key, modifiers);
        }
    }
    true
}

fn step(document: &Rc<RefCell<Document>>, node: NodeId, field: &Field, steps: f64) {
    let fallback = document.borrow().last_committed(node, field);
    let value = field.stepped(steps, fallback);
    if document.borrow_mut().set_number_text(node, written(value)) {
        fire(document, node, "input");
    }
    settle(document, node, value);
}

/// An accessibility set-value (`a11y_set_value`): `text`, a number,
/// becomes the field's text as given; when it changed, `input`, then
/// `change` when the text differs from the last committed.
pub(crate) fn set(document: &Rc<RefCell<Document>>, node: NodeId, text: &str, number: f64) {
    if document.borrow_mut().set_number_text(node, text.to_owned()) {
        fire(document, node, "input");
        settle(document, node, number);
    }
}

/// Commits `node`'s text: its number, in the range or not, or the last
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

/// `value` and the field's value string become the last committed,
/// with `change` when the string differs from the last committed's
/// (`number_committed` cites HTML's rule).
fn settle(document: &Rc<RefCell<Document>>, node: NodeId, value: f64) {
    let now = document
        .borrow()
        .number_field(node)
        .map(|field| field.value());
    let last = document.borrow_mut().remember_committed(node, value);
    if last != now {
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

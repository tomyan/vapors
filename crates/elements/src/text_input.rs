//! A text field's actions (E6a), as Chromium performs them: typed text
//! replaces the selection; Backspace and Delete remove the selection
//! or one cluster (a word with Control); the arrows move by cluster (a
//! word with Control), Home and End to the ends, each extending the
//! selection with Shift; Control+A selects all; Enter commits (the
//! pointer's part is `text_pointer`). Focus by keyboard selects
//! the text whole. `input` fires with each edit, `change` on Enter and
//! blur when the text differs from the last committed, and
//! `selectionchange` once per settle when the selection moved, as
//! Chromium queues one per task.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Event, NodeId};

use crate::document::Document;
use crate::events::tree::dispatch_bubbling;
use crate::keys::Modifiers;

/// The driver a field's editing calls take.
type Driver<'a> = vapors_text::parley::PlainEditorDriver<'a, vapors_scene::Color>;

impl Document {
    /// A text field the user can edit: not disabled.
    pub(crate) fn live_text(&self, node: NodeId) -> bool {
        self.is_text_input(node) && !self.is_disabled_control(node)
    }

    /// Focus arrived at `node` by `keyboard` or not: a text field notes
    /// what it will commit against, selects its text whole on keyboard
    /// focus, and owes a `selectionchange`, as Chromium fires one when
    /// the selection moves into a field.
    pub(crate) fn text_focused(&mut self, node: NodeId, keyboard: bool) {
        if !self.is_text_input(node) {
            return;
        }
        self.editor(node);
        let Document { fonts, editors, .. } = self;
        let editor = editors.get_mut(&node).expect("made above");
        editor.committed = editor.plain.text().to_string();
        if keyboard {
            fonts.editor_driver(&mut editor.plain).select_all();
        }
        editor.pending = true;
        self.changed = true;
    }

    /// Runs `act` on the field's editor through a driver, and marks the
    /// selection as moved.
    pub(crate) fn drive(&mut self, node: NodeId, act: impl FnOnce(&mut Driver<'_>)) {
        self.editor(node);
        let Document { fonts, editors, .. } = self;
        let editor = editors.get_mut(&node).expect("made above");
        act(&mut fonts.editor_driver(&mut editor.plain));
        editor.pending = true;
        self.changed = true;
    }
}

/// `text` was typed into `node`, a focused text field: it replaces the
/// selection.
pub(crate) fn typed(document: &Rc<RefCell<Document>>, node: NodeId, text: &str) {
    if !document.borrow().live_text(node) {
        return;
    }
    document
        .borrow_mut()
        .drive(node, |driver| driver.insert_or_replace_selection(text));
    edited(document, node);
}

/// `key` went down on `node`, a focused text field.
pub(crate) fn key(document: &Rc<RefCell<Document>>, node: NodeId, key: &str, modifiers: Modifiers) {
    if !document.borrow().live_text(node) {
        return;
    }
    let (shift, word) = (modifiers.shift, modifiers.control);
    match key {
        "Enter" => commit(document, node),
        "Backspace" | "Delete" => {
            document.borrow_mut().drive(node, |d| match (key, word) {
                ("Backspace", false) => d.backdelete(),
                ("Backspace", true) => d.backdelete_word(),
                (_, false) => d.delete(),
                (_, true) => d.delete_word(),
            });
            edited(document, node);
        }
        // the method path is not general over the driver's lifetime
        #[allow(clippy::redundant_closure_for_method_calls)]
        "a" | "A" if modifiers.control => document.borrow_mut().drive(node, |d| d.select_all()),
        _ => moved(document, node, key, shift, word),
    }
}

/// An arrow, Home or End moves the caret, extending with `shift`, by
/// words with `word`; another key does nothing.
fn moved(document: &Rc<RefCell<Document>>, node: NodeId, key: &str, shift: bool, word: bool) {
    let act: fn(&mut Driver<'_>) = match (key, shift, word) {
        ("ArrowLeft", false, false) => |d| d.move_left(),
        ("ArrowLeft", true, false) => |d| d.select_left(),
        ("ArrowLeft", false, true) => |d| d.move_word_left(),
        ("ArrowLeft", true, true) => |d| d.select_word_left(),
        ("ArrowRight", false, false) => |d| d.move_right(),
        ("ArrowRight", true, false) => |d| d.select_right(),
        ("ArrowRight", false, true) => |d| d.move_word_right(),
        ("ArrowRight", true, true) => |d| d.select_word_right(),
        ("Home", false, _) => |d| d.move_to_line_start(),
        ("Home", true, _) => |d| d.select_to_line_start(),
        ("End", false, _) => |d| d.move_to_line_end(),
        ("End", true, _) => |d| d.select_to_line_end(),
        _ => return,
    };
    document.borrow_mut().drive(node, act);
}

/// Commits `node`'s text: `change` when it differs from the last
/// committed.
pub(crate) fn commit(document: &Rc<RefCell<Document>>, node: NodeId) {
    if !document.borrow().is_text_input(node) {
        return;
    }
    let changed = {
        let mut doc = document.borrow_mut();
        let editor = doc.editor(node);
        let text = editor.plain.text().to_string();
        let changed = editor.committed != text;
        editor.committed = text;
        changed
    };
    if changed {
        fire(document, node, "change");
    }
}

/// After an edit: the property takes the text and `input` fires.
fn edited(document: &Rc<RefCell<Document>>, node: NodeId) {
    if document.borrow_mut().text_from_editor(node) {
        fire(document, node, "input");
    }
}

/// `input` or `change` on `node`, carrying its text.
fn fire(document: &Rc<RefCell<Document>>, node: NodeId, name: &str) {
    let Some(value) = document.borrow().value(node) else {
        return;
    };
    dispatch_bubbling(document, node, Event::new(name).with_value(&value));
}

impl Document {
    /// Fires the `selectionchange` each text field owes whose selection
    /// moved since the last one, as Chromium fires one per task: the
    /// app calls this before each update, the vapors player after each event.
    pub fn settle_selection(document: &Rc<RefCell<Document>>) {
        let owed: Vec<NodeId> = document
            .borrow_mut()
            .editors
            .iter_mut()
            .filter_map(|(node, editor)| std::mem::take(&mut editor.pending).then_some(*node))
            .collect();
        for node in owed {
            let moved = {
                let mut doc = document.borrow_mut();
                let now = doc.selection(node);
                // a listener of an earlier one may have removed this field
                let Some(editor) = doc.editors.get_mut(&node) else {
                    continue;
                };
                let moved = editor.reported != now;
                editor.reported = now;
                moved && doc.contains(node)
            };
            if moved {
                dispatch_bubbling(document, node, Event::new("selectionchange"));
            }
        }
    }
}

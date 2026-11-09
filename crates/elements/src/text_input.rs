//! A text field's actions (E6a), as Chromium performs them: typed text
//! replaces the selection; Backspace and Delete remove the selection
//! or one cluster (with Control, as far as Control and an arrow moves); the arrows move by cluster (a
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
use crate::text_history::EditKind;

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
        editor.history.close();
        // Chromium fires one as focus arrives, even where the caret was
        editor.reported = None;
        if keyboard {
            fonts.editor_driver(&mut editor.plain).select_all();
        }
        editor.pending = true;
        self.changed = true;
    }

    /// Runs `act`, which moves the selection but edits nothing, on the
    /// field's editor: the open undo step closes.
    pub(crate) fn drive(&mut self, node: NodeId, act: impl FnOnce(&mut Driver<'_>)) {
        self.editor(node).history.close();
        self.drive_editor(node, act);
    }

    /// Runs `act` on the field's editor through a driver, and marks the
    /// selection as moved.
    pub(crate) fn drive_editor(&mut self, node: NodeId, act: impl FnOnce(&mut Driver<'_>)) {
        self.editor(node);
        let Document { fonts, editors, .. } = self;
        let editor = editors.get_mut(&node).expect("made above");
        act(&mut fonts.editor_driver(&mut editor.plain));
        // a composition lasts only through the edits that report it
        editor.composing = None;
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
        .edit(node, EditKind::Insert, |driver| {
            driver.insert_or_replace_selection(text);
        });
    edited(document, node);
}

/// `key` went down on `node`, a focused text field.
/// Whether the field took the key: its editing keys and moves, and a key
/// that types text (`keys::types_text`); a named key such as F1 or
/// Escape is the app's.
pub(crate) fn key(
    document: &Rc<RefCell<Document>>,
    node: NodeId,
    key: &str,
    modifiers: Modifiers,
) -> bool {
    if !document.borrow().live_text(node) {
        return false;
    }
    let (shift, word) = (modifiers.shift, modifiers.control);
    match key {
        "Enter" => commit(document, node),
        "Backspace" | "Delete" => {
            crate::text_delete::delete_key(document, node, key == "Backspace", word);
            edited(document, node);
        }
        _ if crate::shortcuts::selected_all(document, node, key, modifiers) => {}
        _ if crate::text_undo::history_key(document, node, key, modifiers) => {}
        _ => {
            return moved(document, node, key, shift, word)
                || crate::keys::types_text(key, modifiers);
        }
    }
    true
}

/// An arrow, Home or End moves the caret, extending with `shift`, by
/// words with `word`; another key does nothing.
pub(crate) fn moved(
    document: &Rc<RefCell<Document>>,
    node: NodeId,
    key: &str,
    shift: bool,
    word: bool,
) -> bool {
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
        _ => return false,
    };
    document.borrow_mut().drive(node, act);
    true
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
pub(crate) fn edited(document: &Rc<RefCell<Document>>, node: NodeId) {
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

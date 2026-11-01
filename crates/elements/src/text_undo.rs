//! A text field's edits recorded for its undo, and Control+Z, Control+
//! Shift+Z and Control+Y taking them back and forward (E6b), each with
//! `input` when the text changes, as Chromium fires `historyUndo` and
//! `historyRedo` inputs.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::NodeId;

use crate::document::Document;
use crate::text_history::{EditKind, Snapshot};
use crate::text_input::edited;

/// The driver a field's editing calls take.
type Driver<'a> = vapors_text::parley::PlainEditorDriver<'a, vapors_scene::Color>;

impl Document {
    /// Runs `act`, an edit of `kind`, on the field's editor and records
    /// it for undo when it changed the text.
    pub(crate) fn edit(&mut self, node: NodeId, kind: EditKind, act: impl FnOnce(&mut Driver<'_>)) {
        let before = self.snapshot(node);
        self.drive_editor(node, act);
        let after = self.snapshot(node);
        if before.0 != after.0 {
            self.editor(node).history.edited(kind, before, after);
        }
    }

    /// The field's text and selection now.
    fn snapshot(&mut self, node: NodeId) -> Snapshot {
        let editor = &self.editor(node).plain;
        (
            editor.raw_text().to_owned(),
            editor.raw_selection().text_range(),
        )
    }

    /// Puts the field back to `to`: its text, then its selection, which
    /// owes a `selectionchange` even where it was, as Chromium sets it;
    /// but not a caret at the start, which the new text leaves in place
    /// in Chromium too (`undo.events`).
    fn restore(&mut self, node: NodeId, (text, selection): Snapshot) {
        let at_start = selection == (0..0);
        self.drive_editor(node, |d| {
            d.editor.set_text(&text);
            d.select_byte_range(selection.start, selection.end);
        });
        if !at_start {
            self.editor(node).reported = None;
        }
    }
}

/// Control with Z (undo), with Shift and Z, or Y (redo), on `node`, a
/// live text field.
pub(crate) fn history_key(document: &Rc<RefCell<Document>>, node: NodeId, key: &str, shift: bool) {
    let redo = shift || key.eq_ignore_ascii_case("y");
    let to = {
        let mut doc = document.borrow_mut();
        let history = &mut doc.editor(node).history;
        if redo { history.redo() } else { history.undo() }
    };
    if let Some(to) = to {
        document.borrow_mut().restore(node, to);
        edited(document, node);
    }
}

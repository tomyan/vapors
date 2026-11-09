//! Backspace and Delete in a text or number field (E6a), as Chromium
//! deletes: the selection, else a cluster, or with Control a word.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::NodeId;

use crate::document::Document;
use crate::text_history::EditKind;

/// The driver a field's editing calls take.
type Driver<'a> = vapors_text::parley::PlainEditorDriver<'a, vapors_scene::Color>;

/// Backspace (`back`) or Delete, by a word with `word`, on `node`'s
/// editor, as one undo step (`input` is the caller's).
pub(crate) fn delete_key(document: &Rc<RefCell<Document>>, node: NodeId, back: bool, word: bool) {
    let kind = deletion_kind(&document.borrow(), node, back, word);
    document
        .borrow_mut()
        .edit(node, kind, |d| deleted(d, back, word));
}

/// Backspace (`back`) or Delete, by a word with `word`: a selection
/// goes; else a cluster, or with `word` as far as Control+ArrowLeft or
/// Control+ArrowRight moves, as Linux Chromium deletes (Parley's own
/// word deletes skip the space after a word and take "-" with "A").
fn deleted(d: &mut Driver<'_>, back: bool, word: bool) {
    if word && d.editor.raw_selection().is_collapsed() {
        if back {
            d.select_word_left();
        } else {
            d.select_word_right();
        }
    }
    if back {
        d.backdelete();
    } else {
        d.delete();
    }
}

/// How a Backspace (`back`) or Delete groups: a word deletion only
/// when `word` deletes from a collapsed selection, as `deleted` does.
fn deletion_kind(document: &Document, node: NodeId, back: bool, word: bool) -> EditKind {
    let word = word
        && document
            .editors
            .get(&node)
            .is_none_or(|editor| editor.plain.raw_selection().is_collapsed());
    match (back, word) {
        (true, false) => EditKind::DeleteBackward,
        (false, false) => EditKind::DeleteForward,
        (true, true) => EditKind::DeleteWordBackward,
        (false, true) => EditKind::DeleteWordForward,
    }
}

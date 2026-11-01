//! A text field under the pointer (E6a): a press places the caret
//! where it lands, even on a disabled field, as Chromium does, and a
//! drag extends the selection under capture; a double click selects a
//! word, a triple click the whole text.

use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;

use vapors_dom::NodeId;

use crate::document::Document;
use vapors_text::parley::Cursor;

use crate::events::{Asked, Point};

impl Document {
    /// Selects the word a double click at `at` in the field's text
    /// selects (`text_word`).
    fn select_word_at(&mut self, node: NodeId, (x, y): (f32, f32)) {
        self.drive(node, |d| {
            let word = crate::text_word::word_at(d.layout(), x, y);
            d.select_byte_range(word.start, word.end);
        });
    }

    /// Whether `at` in the field's text is on its selection: a range
    /// whose ends hold the caret position nearest `at`, ends included,
    /// as Chromium tests a press.
    fn in_selection(&mut self, node: NodeId, (x, y): (f32, f32)) -> bool {
        let Document { fonts, editors, .. } = self;
        let Some(editor) = editors.get_mut(&node) else {
            return false;
        };
        let range = editor.plain.raw_selection().text_range();
        if range.is_empty() {
            return false;
        }
        let mut driver = fonts.editor_driver(&mut editor.plain);
        let index = Cursor::from_point(driver.layout(), x, y).index();
        (range.start..=range.end).contains(&index)
    }

    /// Where `point` lies in the field's text, from the text's origin.
    fn text_point(&self, node: NodeId, point: Point) -> Option<(f32, f32)> {
        let origin = self.painted.as_ref()?.text_origins.get(&node)?;
        Some((point[0] - origin[0], point[1] - origin[1]))
    }
}

/// A press on a text field, by pointer.
pub(crate) enum TextPress {
    /// A drag that extends the selection from the press.
    Drag(NodeId),
    /// A press inside the selection: the caret goes where it pressed on
    /// release, as Chromium waits to see whether the press drags the
    /// selected text away (dragging text: not built).
    InSelection(NodeId, (f32, f32)),
}

/// Pointer `id` pressed `node` at `point`, the `clicks`th press in a
/// row: a text field, disabled or not, puts its caret there, as Chromium
/// does (on release when the press is inside the selection), and a live
/// one captures the pointer for a drag that extends the selection; a
/// second press selects the word there and a third the whole text, with
/// no drag (by word or line: not built).
pub(crate) fn pressed(
    document: &Rc<RefCell<Document>>,
    id: u32,
    node: NodeId,
    point: Point,
    clicks: NonZeroU32,
) {
    if !document.borrow().is_text_input(node) {
        return;
    }
    let Some(at) = document.borrow().text_point(node, point) else {
        return;
    };
    match clicks.get() {
        1 => first(document, id, node, at),
        2 => document.borrow_mut().select_word_at(node, at),
        // the method path is not general over the driver's lifetime
        #[allow(clippy::redundant_closure_for_method_calls)]
        _ => document.borrow_mut().drive(node, |d| d.select_all()),
    }
}

/// A first press at `at` in the field's text: the caret goes there, now
/// or, inside the selection, on release; a live field captures the
/// pointer.
fn first(document: &Rc<RefCell<Document>>, id: u32, node: NodeId, (x, y): (f32, f32)) {
    let press = if document.borrow_mut().in_selection(node, (x, y)) {
        TextPress::InSelection(node, (x, y))
    } else {
        document.borrow_mut().drive(node, |d| d.move_to_point(x, y));
        TextPress::Drag(node)
    };
    if document.borrow().live_text(node) {
        document.borrow_mut().pointer.text_presses.insert(id, press);
        crate::events::capture::ask_unless_asked(document, id, Asked::CaptureTo(node));
    }
}

/// Pointer `id` moved to `point`: the field it drags extends its
/// selection there.
pub(crate) fn moved_pointer(document: &Rc<RefCell<Document>>, id: u32, point: Point) {
    let dragged = match document.borrow().pointer.text_presses.get(&id) {
        Some(TextPress::Drag(node)) => Some(*node),
        _ => None,
    };
    let Some(node) = dragged else {
        return;
    };
    let Some((x, y)) = document.borrow().text_point(node, point) else {
        return;
    };
    document
        .borrow_mut()
        .drive(node, |d| d.extend_selection_to_point(x, y));
}

/// Pointer `id` came up: a press inside the selection puts the caret
/// where it pressed, and its press of a field ends.
pub(crate) fn released(document: &Rc<RefCell<Document>>, id: u32) {
    let press = document.borrow_mut().pointer.text_presses.remove(&id);
    if let Some(TextPress::InSelection(node, (x, y))) = press {
        document.borrow_mut().drive(node, |d| d.move_to_point(x, y));
    }
}

/// Pointer `id` was taken by the platform: its press of a field ends
/// with nothing moved.
pub(crate) fn dropped(document: &Rc<RefCell<Document>>, id: u32) {
    document.borrow_mut().pointer.text_presses.remove(&id);
}

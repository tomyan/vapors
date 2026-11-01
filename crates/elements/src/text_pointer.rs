//! A text field under the pointer (E6a): a press places the caret
//! where it lands, even on a disabled field, as Chromium does, and a
//! drag extends the selection under capture.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::NodeId;

use crate::document::Document;
use crate::events::{Asked, Point};

impl Document {
    /// Where `point` lies in the field's text, from the text's origin.
    fn text_point(&self, node: NodeId, point: Point) -> Option<(f32, f32)> {
        let origin = self.painted.as_ref()?.text_origins.get(&node)?;
        Some((point[0] - origin[0], point[1] - origin[1]))
    }
}

/// Pointer `id` pressed `node` at `point`: a text field, disabled or
/// not, puts its caret there, as Chromium does, and a live one
/// captures the pointer for a drag that extends the selection.
pub(crate) fn pressed(document: &Rc<RefCell<Document>>, id: u32, node: NodeId, point: Point) {
    if !document.borrow().is_text_input(node) {
        return;
    }
    let Some((x, y)) = document.borrow().text_point(node, point) else {
        return;
    };
    document.borrow_mut().drive(node, |d| d.move_to_point(x, y));
    if document.borrow().live_text(node) {
        document.borrow_mut().pointer.text_drags.insert(id, node);
        crate::events::capture::ask_unless_asked(document, id, Asked::CaptureTo(node));
    }
}

/// Pointer `id` moved to `point`: the field it presses extends its
/// selection there.
pub(crate) fn moved_pointer(document: &Rc<RefCell<Document>>, id: u32, point: Point) {
    let Some(node) = document.borrow().pointer.text_drags.get(&id).copied() else {
        return;
    };
    let Some((x, y)) = document.borrow().text_point(node, point) else {
        return;
    };
    document
        .borrow_mut()
        .drive(node, |d| d.extend_selection_to_point(x, y));
}

/// Pointer `id` came up or was taken: its drag of a field ends.
pub(crate) fn released(document: &Rc<RefCell<Document>>, id: u32) {
    document.borrow_mut().pointer.text_drags.remove(&id);
}

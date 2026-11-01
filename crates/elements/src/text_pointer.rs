//! A text field under the pointer (E6a): a press places the caret
//! where it lands and a drag extends the selection under capture; a
//! press on a disabled field leaves its selection where it was and
//! owes a `selectionchange`, as Chromium does. By touch, a tap places
//! the caret after its click and a finger down or dragging does not, as
//! Chromium's touch does (a drag pans).

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

/// Pointer `id` pressed `node` at `point`: a live text field puts its
/// caret there and captures the pointer for a drag that extends the
/// selection; a disabled one only owes a `selectionchange`.
pub(crate) fn pressed(document: &Rc<RefCell<Document>>, id: u32, node: NodeId, point: Point) {
    if placed(document, node, point) {
        document.borrow_mut().pointer.text_drags.insert(id, node);
        crate::events::capture::ask_unless_asked(document, id, Asked::CaptureTo(node));
    }
}

/// A tap on `node` at `point`: a live text field puts its caret there,
/// with no drag; a disabled one only owes a `selectionchange`.
pub(crate) fn tapped(document: &Rc<RefCell<Document>>, node: NodeId, point: Point) {
    placed(document, node, point);
}

/// Puts a live text field's caret at `point`; whether it did. A disabled
/// field's selection stays where it was and owes a `selectionchange`,
/// as Chromium's does.
fn placed(document: &Rc<RefCell<Document>>, node: NodeId, point: Point) -> bool {
    if !document.borrow().is_text_input(node) {
        return false;
    }
    if !document.borrow().live_text(node) {
        document.borrow_mut().drive(node, |_| {});
        return false;
    }
    let Some((x, y)) = document.borrow().text_point(node, point) else {
        return false;
    };
    document.borrow_mut().drive(node, |d| d.move_to_point(x, y));
    true
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

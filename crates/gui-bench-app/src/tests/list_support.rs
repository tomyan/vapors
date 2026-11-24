//! The `list` scenario mounted headless at the spec window size, driven
//! frame by frame as the app's loop drives it: scrolls settled, then the
//! update.
use super::support::{by_class, rect, text};
use crate::dashboard::WINDOW;
use crate::list::{MountedList, mount};
use vapors_dom::NodeId;
use vapors_elements::{Document, Viewport};

/// A point inside the list, below the header.
pub const IN_LIST: [f32; 2] = [640.0, 400.0];

/// The list of `count` orders, mounted and drawn once, its change flag
/// taken.
pub fn list_of(count: usize) -> MountedList {
    let mounted = mount(count);
    frame(&mounted);
    mounted.document.borrow_mut().take_changed();
    mounted
}

/// What a frame does: settle the scrolls owed, then update.
pub fn frame(mounted: &MountedList) {
    Document::settle_scroll(&mounted.document);
    let viewport = Viewport {
        width: WINDOW.0,
        height: WINDOW.1,
        scale: 1.0,
    };
    mounted
        .document
        .borrow_mut()
        .update(viewport)
        .expect("the list lays out");
}

/// A wheel scroll of `dy` px over the list, then a frame.
pub fn wheel(mounted: &MountedList, dy: f32) {
    let moved = mounted.document.borrow_mut().scroll_by(IN_LIST, [0.0, dy]);
    assert!(moved, "the wheel moved the list");
    frame(mounted);
}

/// A click at `point`, then a frame.
pub fn click(mounted: &MountedList, point: [f32; 2]) {
    Document::pointer_moved(&mounted.document, point);
    Document::pointer_pressed(&mounted.document, point);
    Document::pointer_released(&mounted.document, point);
    frame(mounted);
}

/// The texts of every element with class `class`, in document order.
pub fn texts(mounted: &MountedList, class: &str) -> Vec<String> {
    let doc = mounted.document.borrow();
    let nodes = by_class(&doc, class);
    nodes.into_iter().map(|node| text(&doc, node)).collect()
}

/// The order rows that exist, in document order.
pub fn rows(mounted: &MountedList) -> Vec<NodeId> {
    by_class(&mounted.document.borrow(), "order-row")
}

/// The centre of the first element with class `class`.
pub fn centre_of(mounted: &MountedList, class: &str) -> [f32; 2] {
    let doc = mounted.document.borrow();
    let [x, y, w, h] = rect(&doc, by_class(&doc, class)[0]);
    [x + w / 2.0, y + h / 2.0]
}

/// The order number shown in the row under window `y`.
pub fn number_at(mounted: &MountedList, y: f32) -> String {
    let doc = mounted.document.borrow();
    let painted = doc.painted().expect("drawn");
    let mut node = painted.hit([30.0, y]).expect("something at y");
    while doc.attribute(node, "class") != Some("order-row") {
        node = doc.parent(node).expect("inside an order row");
    }
    let cells = by_class(&doc, "c-number");
    let cell = cells
        .into_iter()
        .find(|cell| doc.parent(*cell) == Some(node));
    text(&doc, cell.expect("the row's number cell"))
}

/// The numbers of orders `indices`, as a row shows them.
pub fn numbers(indices: impl IntoIterator<Item = usize>) -> Vec<String> {
    indices
        .into_iter()
        .map(|i| format!("#{}", 10200 + i))
        .collect()
}

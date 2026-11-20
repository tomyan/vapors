//! Finding the dashboard's elements and reading their boxes.
use crate::app::{Mounted, mount_with};
use crate::dashboard::WINDOW;
use vapors_dom::NodeId;
use vapors_elements::{Document, Viewport};

pub type Rect = [f32; 4];

/// The dashboard mounted and laid out at the spec's window size.
pub fn laid_out() -> Mounted {
    let mounted = mount_with(|_| {});
    refresh(&mounted);
    mounted
}

pub fn refresh(mounted: &Mounted) {
    let viewport = Viewport {
        width: WINDOW.0,
        height: WINDOW.1,
        scale: 1.0,
    };
    mounted
        .document
        .borrow_mut()
        .update(viewport)
        .expect("the dashboard lays out");
}

/// Every element with class `class`, in document order.
pub fn by_class(doc: &Document, class: &str) -> Vec<NodeId> {
    let has = |node: &NodeId| {
        doc.attribute(*node, "class")
            .is_some_and(|names| names.split(' ').any(|name| name == class))
    };
    doc.elements().into_iter().filter(has).collect()
}

/// The border box of `node`.
pub fn rect(doc: &Document, node: NodeId) -> Rect {
    let found = doc
        .painted()
        .and_then(|painted| painted.border_box(node))
        .expect("laid out");
    [found.x, found.y, found.width, found.height]
}

/// The text inside `node`, its descendants' joined.
pub fn text(doc: &Document, node: NodeId) -> String {
    let mut out = String::new();
    for child in doc.children(node) {
        out += doc.text(child).unwrap_or_default();
        out += &text(doc, child);
    }
    out
}

/// A click: the pointer moves onto `point`, presses and releases, and the
/// document lays out again.
pub fn click(mounted: &Mounted, point: [f32; 2]) {
    Document::pointer_moved(&mounted.document, point);
    Document::pointer_pressed(&mounted.document, point);
    Document::pointer_released(&mounted.document, point);
    refresh(mounted);
}

/// The centre of the `nth` element of `class`.
pub fn centre_of(mounted: &Mounted, class: &str, nth: usize) -> [f32; 2] {
    let doc = mounted.document.borrow();
    let [x, y, w, h] = rect(&doc, by_class(&doc, class)[nth]);
    [x + w / 2.0, y + h / 2.0]
}

/// The text of every element of `class`, in document order.
pub fn texts(mounted: &Mounted, class: &str) -> Vec<String> {
    let doc = mounted.document.borrow();
    let nodes = by_class(&doc, class);
    nodes.into_iter().map(|node| text(&doc, node)).collect()
}

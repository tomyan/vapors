//! A document with two nested scroll containers, for the app's scroll
//! tests.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_text::FontSystem;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 200.0,
    scale: 1.0,
};

/// `.outer` (100 px, 200 px of content) holding `.inner` (60 px tall,
/// 100 px of content) at its top.
pub(crate) fn nested() -> (Rc<RefCell<Document>>, NodeId, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    let element = |doc: &mut Document, parent: NodeId, class: &str| {
        let node = doc.create_element("div");
        doc.set_attribute(node, "class", class);
        doc.append(parent, node);
        node
    };
    let outer = element(&mut doc, body, "outer");
    let inner = element(&mut doc, outer, "inner");
    element(&mut doc, inner, "filler");
    element(&mut doc, outer, "tail");
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } .outer { height: 100px; overflow: auto; } \
         .inner { height: 60px; overflow: auto; } .filler { height: 100px; } .tail { height: 140px; }",
    )
    .expect("parses");
    doc.update(VIEWPORT).expect("updates");
    (Rc::new(RefCell::new(doc)), outer, inner)
}

pub(crate) fn offset(doc: &Rc<RefCell<Document>>, node: NodeId) -> f32 {
    doc.borrow().scroll_offset(node).expect("scrolls")[1]
}

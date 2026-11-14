//! The pages the lane tests lay out: a stylesheet and divs
//! under body, updated once.

use vapors_dom::{Dom, NodeId as DomNode};
use vapors_text::FontSystem;

use crate::document::Document;
use crate::update::Viewport;

pub(crate) const VIEWPORT: Viewport = Viewport {
    width: 300.0,
    height: 200.0,
    scale: 1.0,
};

/// A page styled by `css`, built under body by `build`, laid out once.
pub(crate) fn page(css: &str, build: impl FnOnce(&mut Document, DomNode)) -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    build(&mut doc, body);
    doc.add_stylesheet("page.css", css).expect("parses");
    doc.boxes.lanes_off = true;
    doc.update(VIEWPORT).expect("updates");
    doc
}

/// `div.class` under `parent`, with `text` in it when given.
pub(crate) fn element(
    doc: &mut Document,
    parent: DomNode,
    class: &str,
    text: Option<&str>,
) -> DomNode {
    let node = doc.create_element("div");
    doc.set_attribute(node, "class", class);
    doc.append(parent, node);
    if let Some(text) = text {
        let text = doc.create_text(text);
        doc.append(node, text);
    }
    node
}

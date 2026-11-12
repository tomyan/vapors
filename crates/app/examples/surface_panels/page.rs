//! The page: two panels side by side, each a title over a `gpu-surface`.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;
use vapors_text::FontSystem;

/// The surfaces' names, left then right.
pub const SURFACES: [&str; 2] = ["left", "right"];

const STYLE: &str = "html, body { margin: 0; height: 100%; background-color: #20242a; } \
    body { display: flex; } \
    .panel { flex-grow: 1; display: flex; flex-direction: column; margin: 16px; \
    overflow: hidden; color: #e6e6e6; font-size: 14px; } \
    .title { margin-bottom: 8px; } \
    .view { flex-grow: 1; }";

const TITLES: [&str; 2] = [
    "Animating on every frame",
    "Animating for 2 s, then stopped for 2 s",
];

/// The page, styled.
pub fn document() -> Rc<RefCell<Document>> {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("surface_panels.css", STYLE)
        .expect("the example's sheet parses");
    for (name, title) in SURFACES.into_iter().zip(TITLES) {
        panel(&mut doc, name, title);
    }
    Rc::new(RefCell::new(doc))
}

fn panel(doc: &mut Document, name: &str, title: &str) {
    let body = doc.body();
    let panel = element(doc, body, "div", "panel");
    let heading = element(doc, panel, "div", "title");
    let words = doc.create_text(title);
    doc.append(heading, words);
    let view = element(doc, panel, "gpu-surface", "view");
    doc.set_attribute(view, "name", name);
}

fn element(doc: &mut Document, parent: NodeId, tag: &str, class: &str) -> NodeId {
    let made = doc.create_element(tag);
    doc.set_attribute(made, "class", class);
    doc.append(parent, made);
    made
}

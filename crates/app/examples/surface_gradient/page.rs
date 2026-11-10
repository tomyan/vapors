//! The page: a side panel of ordinary UI and the `gpu-surface` the
//! gradient draws in.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;
use vapors_text::FontSystem;

/// The surface's name, which the renderer is mounted under.
pub const SURFACE: &str = "gradient";

const STYLE: &str = "html, body { margin: 0; height: 100%; background-color: #20242a; } \
    body { display: flex; } \
    .panel { width: 200px; padding: 16px; color: #e6e6e6; font-size: 14px; } \
    .title { font-size: 20px; margin-bottom: 8px; } \
    .view { flex-grow: 1; }";

/// The page, styled.
pub fn document() -> Rc<RefCell<Document>> {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("surface_gradient.css", STYLE)
        .expect("the example's sheet parses");
    let body = doc.body();
    let panel = element(&mut doc, body, "div", "panel");
    let title = element(&mut doc, panel, "div", "title");
    text(&mut doc, title, "Gradient");
    let note = element(&mut doc, panel, "div", "note");
    text(
        &mut doc,
        note,
        "Drawn by a WGSL renderer in a gpu-surface, every frame while the app animates it.",
    );
    let view = element(&mut doc, body, "gpu-surface", "view");
    doc.set_attribute(view, "name", SURFACE);
    Rc::new(RefCell::new(doc))
}

fn element(doc: &mut Document, parent: NodeId, tag: &str, class: &str) -> NodeId {
    let made = doc.create_element(tag);
    doc.set_attribute(made, "class", class);
    doc.append(parent, made);
    made
}

fn text(doc: &mut Document, parent: NodeId, words: &str) {
    let made = doc.create_text(words);
    doc.append(parent, made);
}

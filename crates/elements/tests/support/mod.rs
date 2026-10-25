//! Shared by the elements tests: a document on the bundled fonts only,
//! so results do not depend on the machine's fonts.

#![allow(dead_code)] // each test file uses a subset

pub mod changes;
pub mod pointer;
pub mod random_css;
pub mod scroll_list;
pub mod tree;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, ElementError, Viewport};
use vapors_text::FontSystem;

/// An empty document laying text out with Geist and Geist Mono only.
pub fn document() -> Document {
    Document::new(FontSystem::bundle_only())
}

/// A 1x viewport of `width` by `height` CSS pixels.
pub fn viewport(width: f32, height: f32) -> Viewport {
    Viewport {
        width,
        height,
        scale: 1.0,
    }
}

/// An element with `tag` and `class`, appended to `parent`.
pub fn element(doc: &mut Document, parent: NodeId, tag: &str, class: &str) -> NodeId {
    let node = doc.create_element(tag);
    if !class.is_empty() {
        doc.set_attribute(node, "class", class);
    }
    doc.append(parent, node);
    node
}

/// A text node appended to `parent`.
pub fn text(doc: &mut Document, parent: NodeId, text: &str) -> NodeId {
    let node = doc.create_text(text);
    doc.append(parent, node);
    node
}

/// The update's errors, each as its message; panics when it succeeds.
pub fn refusals(doc: &mut Document) -> Vec<String> {
    match doc.update(viewport(400.0, 300.0)) {
        Ok(_) => panic!("the update succeeded; a refusal was expected"),
        Err(errors) => errors.iter().map(ElementError::to_string).collect(),
    }
}

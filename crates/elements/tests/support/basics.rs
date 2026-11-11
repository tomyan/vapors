//! The nodes every elements test page starts from: a document on the
//! bundled fonts only, a 1x viewport, and element and text nodes. Its
//! own file, with no `super::`, so `crates/perf` includes it by path
//! beside `rows_page.rs` and the workload's fonts are defined once.

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_text::FontSystem;

/// An empty document laying text out with Geist and Geist Mono only,
/// its shortcuts on Control (Linux's, as the tests' Control keys expect).
pub fn document() -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.set_shortcut_modifier(vapors_elements::ShortcutModifier::Control);
    doc
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

//! Every frame a paint holds is of the class of the frame it is in,
//! the page's a whole translation (`frames.rs` `FrameAt::class`).

use vapors_dom::{Dom, NodeId};
use vapors_scene::TransformClass;
use vapors_text::FontSystem;

use super::PAGE;
use crate::document::Document;
use crate::update::Viewport;

/// A fractional scale, so shifts are scaled CSS pixels made whole; every
/// height below is whole at it, so the list's lane is framed.
const VIEWPORT: Viewport = Viewport {
    width: 300.0,
    height: 200.0,
    scale: 2.75,
};

const CSS: &str = "body { margin: 0; } .row { height: 20px; } \
    .scroll { overflow: auto; height: 20px; } .tall { height: 60px; } \
    .top { overflow: auto; height: 40px; }";

/// An element of class `class` appended to `parent`.
fn child(doc: &mut Document, parent: NodeId, class: &str) -> NodeId {
    let node = doc.create_element("div");
    doc.set_attribute(node, "class", class);
    doc.append(parent, node);
    node
}

/// A scroller at the top, then a framed list of 40 rows, each a
/// scroller over a tall child: scroll frames on the page, lane frames,
/// and scroll frames under lane frames. Each scroller scrolled.
fn every_kind_of_frame() -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("p.css", CSS).expect("parses");
    let body = doc.body();
    let top = child(&mut doc, body, "top");
    child(&mut doc, top, "tall");
    let list = child(&mut doc, body, "list");
    let scrollers: Vec<NodeId> = (0..40)
        .map(|_| {
            let row = child(&mut doc, list, "row");
            let scroller = child(&mut doc, row, "scroll");
            child(&mut doc, scroller, "tall");
            scroller
        })
        .collect();
    doc.update(VIEWPORT).expect("updates");
    for &scroller in std::iter::once(&top).chain(&scrollers) {
        assert!(doc.scroll_node_by(scroller, [0.0, 5.0]));
    }
    doc.update(VIEWPORT).expect("updates");
    doc
}

#[test]
fn every_frame_is_a_whole_translation_as_the_page_is() {
    // Given
    let doc = every_kind_of_frame();
    let painted = doc.painted.as_ref().expect("painted");

    // When
    let frames: Vec<_> = (0..painted.kept_boxes.len())
        .chain([PAGE])
        .filter_map(|slot| painted.frames.get(slot))
        .collect();

    // Then: the page, the top scroller, 40 lane frames and the 40
    // scroll frames under them
    assert_eq!(frames.len(), 82);
    assert_eq!(painted.frames.relative.len(), 40);
    for at in frames {
        assert_eq!(at.class, TransformClass::WholeTranslate, "{at:?}");
    }
}

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

/// A framed list of 100 rows after a row is put in at the top: every
/// later row moved, and no frame of theirs was read this update.
fn rows_after_an_insert() -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "body { margin: 0; } .row { height: 20px; }";
    doc.add_stylesheet("p.css", css).expect("parses");
    let body = doc.body();
    let list = child(&mut doc, body, "list");
    let first = (0..100)
        .map(|_| child(&mut doc, list, "row"))
        .collect::<Vec<_>>()[0];
    doc.update(VIEWPORT).expect("updates");
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    doc.insert_before(list, row, Some(first));
    doc.update(VIEWPORT).expect("updates");
    doc
}

#[test]
fn a_lane_frames_class_is_read_without_folding_the_frame() {
    // Given: the lane frames of rows nothing read this update
    let doc = rows_after_an_insert();
    let frames = &doc.painted.as_ref().expect("painted").frames;
    let rows: Vec<usize> = (0..doc.painted.as_ref().expect("painted").kept_boxes.len())
        .filter(|&slot| frames.lane_parent(slot).is_some())
        .collect();
    let before = frames.lane_folds();

    // When
    let classes: Vec<_> = rows.iter().map(|&row| frames.class_of(row)).collect();

    // Then: each a whole translation, and no frame folded for it; a
    // read of the frames themselves folds them
    assert!(rows.len() >= 100, "{} lane frames", rows.len());
    assert!(
        classes
            .iter()
            .all(|&class| class == TransformClass::WholeTranslate)
    );
    assert_eq!(
        frames.lane_folds(),
        before,
        "reading a class folds its frame"
    );
    for &row in &rows {
        let _ = frames.at(row);
    }
    assert!(frames.lane_folds() > before, "the frames fold when read");
}

use vapors_dom::{Dom, NodeId};
use vapors_text::FontSystem;

use crate::document::Document;
use crate::update::Viewport;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

/// A scroll container holding a scroll container, each with content
/// taller than it, updated once; the outer one.
fn nested_scrollers() -> (Document, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "app.css",
        ".outer { overflow: auto; height: 40px; } .inner { overflow: auto; height: 20px; } \
         .tall { height: 80px; }",
    )
    .expect("parses");
    let body = doc.body();
    let [outer, inner, tall, inner_tall] = ["div"; 4].map(|tag| doc.create_element(tag));
    for (node, class) in [
        (outer, "outer"),
        (inner, "inner"),
        (tall, "tall"),
        (inner_tall, "tall"),
    ] {
        doc.set_attribute(node, "class", class);
    }
    doc.append(body, outer);
    doc.append(outer, tall);
    doc.append(tall, inner);
    doc.append(inner, inner_tall);
    doc.update(VIEWPORT).expect("updates");
    (doc, outer)
}

#[test]
fn a_scroll_works_out_the_scrolled_frame_and_the_frame_nested_in_it() {
    // Given
    let (mut doc, outer) = nested_scrollers();

    // When: a debug build compares the kept frames with fresh ones
    assert!(doc.scroll_node_by(outer, [0.0, 10.0]));
    let painted = doc.update(VIEWPORT).expect("updates");

    // Then
    assert_eq!(painted.frames_worked, 2);
}

/// Five scroll containers, each in a block that hides its overflow,
/// updated once; the last block.
fn clipped_scrollers() -> (Document, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "app.css",
        ".clip { overflow: hidden; height: 50px; } .small { height: 30px; } \
         .scroller { overflow: auto; height: 40px; } .tall { height: 80px; }",
    )
    .expect("parses");
    let body = doc.body();
    let mut last = body;
    for _ in 0..5 {
        let [clip, scroller, tall] = ["div"; 3].map(|tag| doc.create_element(tag));
        for (node, class) in [(clip, "clip"), (scroller, "scroller"), (tall, "tall")] {
            doc.set_attribute(node, "class", class);
        }
        doc.append(body, clip);
        doc.append(clip, scroller);
        doc.append(scroller, tall);
        last = clip;
    }
    doc.update(VIEWPORT).expect("updates");
    (doc, last)
}

#[test]
fn a_clip_cut_by_a_resized_ancestor_works_out_that_frame_alone() {
    // Given
    let (mut doc, last) = clipped_scrollers();

    // When: the last block shrinks to 30 px, cutting its scroller's clip
    doc.set_attribute(last, "class", "clip small");
    let painted = doc.update(VIEWPORT).expect("updates");

    // Then
    assert_eq!(painted.frames_worked, 1);
}

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

/// A 40 px scroll container below a 200 px block, holding a surface,
/// updated once in the 200 x 100 viewport: the container's clip is
/// outside the view.
fn surface_below_the_view() -> (Document, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } .spacer { height: 200px; } \
         .scroller { overflow: auto; height: 40px; } gpu-surface { display: block; height: 80px; } \
         .hidden { visibility: hidden; }",
    )
    .expect("parses");
    let body = doc.body();
    let [spacer, scroller, surface] =
        ["div", "div", "gpu-surface"].map(|tag| doc.create_element(tag));
    doc.set_attribute(spacer, "class", "spacer");
    doc.set_attribute(scroller, "class", "scroller");
    doc.set_attribute(surface, "name", "track");
    doc.append(body, spacer);
    doc.append(body, scroller);
    doc.append(scroller, surface);
    doc.update(VIEWPORT).expect("updates");
    (doc, surface)
}

#[test]
fn a_surface_in_a_frame_outside_the_view_keeps_its_culled_batch() {
    // Given
    let (doc, _) = surface_below_the_view();
    let painted = doc.painted.as_ref().expect("painted");

    // When: a debug build compares the kept batches with batching the
    // whole scene afresh, which keeps the culled surface's batch
    let view = vapors_scene::Rect::new(0.0, 0.0, 200.0, 100.0);
    let batches = painted.kept_batches(view);

    // Then
    let culled = batches.batched.batches.iter().filter(|batch| batch.culled);
    assert_eq!(culled.count(), 1);
}

#[test]
fn a_surface_hidden_in_a_frame_leaves_no_frame_searched_for_it() {
    // Given
    let (mut doc, surface) = surface_below_the_view();

    // When: hidden, so its step is patched out of the extents index; a
    // debug build compares the index, with the frames holding a step
    // that spans every view, with one made afresh
    doc.set_attribute(surface, "class", "hidden");
    let painted = doc.update(VIEWPORT).expect("updates");

    // Then
    assert_eq!(painted.extents.spanning_frames().count(), 0);
}

//! `overflow: hidden` and `clip` (F2c S8) beyond what the `clipping`
//! paint fixture compares with Chromium: what a clip takes from hit
//! testing, and how `html` and `body` pass theirs to the viewport.

mod support;

use support::{document, element, text, viewport};
use vapors_dom::NodeId;
use vapors_elements::Document;

/// `.frame` (50 x 50 at the origin, with `frame`) holding `.wide`
/// (100 x 20, with `wide`); the point (75, 10) is on `.wide` outside
/// the frame.
fn framed(frame: &str, wide: &str) -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let outer = element(&mut doc, body, "div", "frame");
    let inner = element(&mut doc, outer, "div", "wide");
    let css = format!(
        "body {{ margin: 0; }} .frame {{ width: 50px; height: 50px; {frame} }} \
         .wide {{ width: 100px; height: 20px; {wide} }}"
    );
    doc.add_stylesheet("app.css", &css).expect("parses");
    (doc, inner)
}

fn hit(doc: &mut Document, point: [f32; 2]) -> Option<NodeId> {
    doc.update(viewport(200.0, 200.0))
        .expect("updates")
        .hit(point)
}

#[test]
fn an_overflowing_child_is_hit_outside_a_visible_parent() {
    // Given
    let (mut doc, wide) = framed("", "");

    // When / Then
    assert_eq!(hit(&mut doc, [75.0, 10.0]), Some(wide));
}

#[test]
fn hidden_and_clip_cut_a_child_off_at_the_parent() {
    for overflow in ["hidden", "clip"] {
        // Given
        let (mut doc, wide) = framed(&format!("overflow: {overflow};"), "");

        // When / Then
        assert_ne!(hit(&mut doc, [75.0, 10.0]), Some(wide), "{overflow}");
        assert_eq!(hit(&mut doc, [25.0, 10.0]), Some(wide), "{overflow}");
    }
}

#[test]
fn the_clip_is_the_padding_box() {
    // Given: a 5 px border; the child starts under it
    let (mut doc, wide) = framed(
        "overflow: hidden; border: 5px solid black;",
        "margin: -5px;",
    );

    // When / Then: on the border, outside the padding box
    assert_ne!(hit(&mut doc, [2.0, 10.0]), Some(wide));
    assert_eq!(hit(&mut doc, [6.0, 10.0]), Some(wide));
}

#[test]
fn an_absolute_child_placed_by_the_clipping_box_is_clipped() {
    // Given
    let (mut doc, wide) = framed(
        "overflow: hidden; position: relative;",
        "position: absolute;",
    );

    // When / Then: cut off past the frame, hit inside it
    assert_ne!(hit(&mut doc, [75.0, 10.0]), Some(wide));
    assert_eq!(hit(&mut doc, [25.0, 10.0]), Some(wide));
}

#[test]
fn an_absolute_child_placed_by_a_box_outside_the_clip_is_not_clipped() {
    // Given: the frame clips but is static; the body places the child
    let (mut doc, wide) = framed("overflow: hidden;", "position: absolute;");

    // When / Then
    assert_eq!(hit(&mut doc, [75.0, 10.0]), Some(wide));
}

#[test]
fn a_fixed_child_is_not_clipped() {
    // Given
    let (mut doc, wide) = framed(
        "overflow: hidden; position: relative;",
        "position: fixed; top: 0; left: 0;",
    );

    // When / Then
    assert_eq!(hit(&mut doc, [75.0, 10.0]), Some(wide));
}

#[test]
fn nested_clips_cut_to_both() {
    // Given: a 100 x 60 child in an 80 x 30 clip in a 50 x 50 frame;
    // each clip reaches past the other on one axis
    let mut doc = document();
    let body = doc.body();
    let frame = element(&mut doc, body, "div", "frame");
    let band = element(&mut doc, frame, "div", "band");
    let big = element(&mut doc, band, "div", "big");
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } .frame { width: 50px; height: 50px; overflow: hidden; } \
         .band { width: 80px; height: 30px; overflow: hidden; } .big { width: 100px; height: 60px; }",
    )
    .expect("parses");

    // When / Then: inside both; past the frame; past the band
    assert_eq!(hit(&mut doc, [40.0, 10.0]), Some(big));
    assert_ne!(hit(&mut doc, [60.0, 10.0]), Some(big));
    assert_eq!(hit(&mut doc, [40.0, 40.0]), Some(frame));
}

#[test]
fn a_hidden_body_passes_its_clip_to_the_viewport() {
    // Given: the body is 20 px tall; its child is 100 px tall
    let mut doc = document();
    let body = doc.body();
    let tall = element(&mut doc, body, "div", "tall");
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; height: 20px; overflow: hidden; } .tall { height: 100px; }",
    )
    .expect("parses");

    // When / Then: below the body, the child is still hit
    assert_eq!(hit(&mut doc, [10.0, 50.0]), Some(tall));
}

#[test]
fn a_hidden_body_clips_when_the_root_passes_its_own() {
    // Given: html and body both hidden; the root's goes to the viewport
    let mut doc = document();
    let body = doc.body();
    let tall = element(&mut doc, body, "div", "tall");
    doc.add_stylesheet(
        "app.css",
        "html { overflow: hidden; } body { margin: 0; height: 20px; overflow: hidden; } .tall { height: 100px; }",
    )
    .expect("parses");

    // When / Then: cut off below the body, hit inside it
    assert_ne!(hit(&mut doc, [10.0, 50.0]), Some(tall));
    assert_eq!(hit(&mut doc, [10.0, 10.0]), Some(tall));
}

#[test]
fn an_inline_span_is_not_hit_where_it_is_cut_off() {
    // Given: a span of one unbreakable word running past a 50 px frame
    let mut doc = document();
    let body = doc.body();
    let frame = element(&mut doc, body, "div", "frame");
    let span = element(&mut doc, frame, "span", "long");
    text(
        &mut doc,
        span,
        "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
    );
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } .frame { width: 50px; height: 50px; overflow: hidden; }",
    )
    .expect("parses");

    // When / Then
    assert_eq!(hit(&mut doc, [10.0, 8.0]), Some(span));
    assert_ne!(hit(&mut doc, [120.0, 8.0]), Some(span));
}

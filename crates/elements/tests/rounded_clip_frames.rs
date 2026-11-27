//! What a framed lane under a rounded clip refuses of the boxes in its
//! rows (`docs/plans/frame-clips.md`, slice 2, `clip_refusals.rs`): an
//! img, a gpu-surface and an svg drawing shapes, which draw under the
//! scissor; a framed lane that does not clip; and a scroller not held
//! clear of the rounded frame's corners, checked every update, as rows
//! move with none of their boxes painted again. A scroller held clear is
//! accepted.

use crate::support;

use support::rounded_page::{at, card_list};
use support::svg_page::{SVG, with_attributes};
use support::{element, refusals};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// Asserts `errors` hold one naming `what`.
fn assert_refused(errors: &[String], what: &str) {
    assert!(
        errors.iter().any(|e| e.contains(what)),
        "{what}: {errors:?}"
    );
}

#[test]
fn an_img_in_a_rounded_frame_is_refused() {
    // Given: an image in the third row of a framed lane
    let (mut doc, _, rows) = card_list("img { display: block; }");
    let image = vapors_scene::RgbaImage {
        size: vapors_scene::Size {
            width: 2,
            height: 2,
        },
        rgba: [0, 0, 0, 255].repeat(4),
    };
    doc.register_image("a.png", image).expect("registers");
    let img = element(&mut doc, rows[2], "img", "");
    doc.set_attribute(img, "src", "a.png");

    // Then
    assert_refused(&refusals(&mut doc), "is an img under a rounded clip");
}

#[test]
fn a_gpu_surface_in_a_rounded_frame_is_refused() {
    // Given
    let (mut doc, _, rows) = card_list("gpu-surface { display: block; height: 4px; }");
    let surface = element(&mut doc, rows[2], "gpu-surface", "");
    doc.set_attribute(surface, "name", "track");

    // Then
    assert_refused(&refusals(&mut doc), "is a gpu-surface under a rounded clip");
}

/// An svg drawing a rect, appended to `parent`.
fn svg_in(doc: &mut Document, parent: NodeId) {
    let svg = element(doc, parent, "svg", "icon");
    with_attributes(doc, svg, SVG);
    let rect = element(doc, svg, "rect", "");
    with_attributes(
        doc,
        rect,
        &[("width", "8"), ("height", "8"), ("fill", "red")],
    );
}

#[test]
fn an_svg_drawing_shapes_in_a_rounded_frame_is_refused() {
    // Given
    let (mut doc, _, rows) = card_list(".icon { display: block; }");
    svg_in(&mut doc, rows[2]);

    // Then
    assert_refused(
        &refusals(&mut doc),
        "is an svg drawing shapes under a rounded clip",
    );
}

#[test]
fn an_svg_drawing_shapes_under_a_rounded_item_clip_is_refused() {
    // Given: the svg in the card directly, under its rounded clip as an
    // item, not in a frame
    let (mut doc, card) = support::rounded_page::card(".icon { display: block; }", "");
    svg_in(&mut doc, card);

    // Then
    assert_refused(
        &refusals(&mut doc),
        "is an svg drawing shapes under a rounded clip",
    );
}

#[test]
fn a_framed_lane_that_does_not_clip_in_a_rounded_frame_is_refused() {
    // Given: the third row holds a list of 40 rows of its own, framed, and
    // not clipping, so its rows reach past its box
    let (mut doc, _, rows) = card_list(".tiny { height: 1px; }");
    let inner = element(&mut doc, rows[2], "div", "inner");
    for _ in 0..40 {
        element(&mut doc, inner, "div", "tiny");
    }

    // Then: named with the fix that works
    assert_refused(
        &refusals(&mut doc),
        "give this box overflow: hidden or clip",
    );
}

#[test]
fn a_scroller_in_a_rounded_frame_clear_of_its_corners_is_accepted() {
    // Given: the first row, across the card's top corners, holds a
    // scroller inset from its sides, clear of the 11 px curves
    let (mut doc, _, rows) = card_list(".pane { overflow: auto; height: 10px; margin: 0 20px; }");
    element(&mut doc, rows[0], "div", "pane");

    // Then: the update is not refused
    doc.update(at(1.0)).expect("updates");
}

#[test]
fn a_scroller_in_a_rounded_frame_crossing_its_corners_is_refused() {
    // Given: the first row holds a scroller as wide as the card, across
    // its top corners
    let (mut doc, _, rows) = card_list(".pane { overflow: auto; height: 10px; }");
    element(&mut doc, rows[0], "div", "pane");

    // When
    let errors = refusals(&mut doc);

    // Then: named, with the box that rounds the frame
    assert_refused(&errors, "inside a frame with a rounded clip");
    assert_refused(&errors, "(in the frame rounded by <div class=\"card\">)");
}

#[test]
fn a_scroller_moved_into_a_rounded_frames_corner_by_rows_above_it_is_refused() {
    // Given: a scroller as wide as the card in the fourth row, y 31..41,
    // between the card's corners, updated
    let (mut doc, _, rows) = card_list(".pane { overflow: auto; height: 10px; }");
    element(&mut doc, rows[3], "div", "pane");
    doc.update(at(1.0)).expect("updates");

    // When: two rows put in above it move it to y 51..61, across the
    // bottom corners, painting none of its boxes again
    let list = doc.parent(rows[0]).expect("in the list");
    for _ in 0..2 {
        let row = doc.create_element("div");
        doc.set_attribute(row, "class", "row");
        doc.insert_before(list, row, Some(rows[0]));
    }

    // Then
    assert_refused(&refusals(&mut doc), "inside a frame with a rounded clip");
}

#[test]
fn an_img_in_a_lane_row_is_refused_once_the_card_rounds_and_not_after() {
    // Given: a square card's framed lane, an image in its first row,
    // updated
    let (mut doc, card, rows) = card_list(".square { border-radius: 0; } img { display: block; }");
    doc.set_attribute(card, "class", "card square");
    let image = vapors_scene::RgbaImage {
        size: vapors_scene::Size {
            width: 2,
            height: 2,
        },
        rgba: [0, 0, 0, 255].repeat(4),
    };
    doc.register_image("a.png", image).expect("registers");
    let img = element(&mut doc, rows[0], "img", "");
    doc.set_attribute(img, "src", "a.png");
    doc.update(at(1.0)).expect("updates");

    // When: the card rounds
    doc.set_attribute(card, "class", "card");

    // Then: refused; and accepted again once it is square
    assert_refused(&refusals(&mut doc), "is an img under a rounded clip");
    doc.set_attribute(card, "class", "card square");
    doc.update(at(1.0)).expect("updates");
}

//! `<rect>` and `<circle>` in an svg (one-scene S4b): drawn on the
//! shapes route as closed forms, never tessellated, a pill and a full
//! ellipse included. One cache slot is held for each distinct shape. A
//! rect paints in document order with paths. A size of 0 draws nothing,
//! as in SVG and Chromium. What they refuse is in
//! `svg_basic_shape_refusals.rs`.

mod support;

use support::svg_shapes_page::{CIRCLE, RECT, drawn, page, rgb};
use vapors_dom::Dom;
use vapors_scene::ShapeKind;

#[test]
fn a_rect_with_elliptical_corners_and_a_circle_are_closed_forms_never_tessellated() {
    // Given
    let (mut doc, _) = page(&[&[("rect", RECT), ("circle", CIRCLE)]]);

    // When
    let shapes = drawn(&mut doc);

    // Then
    let closed = ShapeKind::ClosedForm;
    assert_eq!(shapes, [(closed, rgb(255, 0, 0)), (closed, rgb(0, 0, 255))]);
    assert_eq!(doc.update_counts().shapes_tessellated, 0);
}

#[test]
fn two_svgs_with_the_same_rect_hold_one_slot() {
    // Given
    let (mut doc, _) = page(&[&[("rect", RECT)], &[("rect", RECT)]]);

    // When
    let shapes = drawn(&mut doc);

    // Then
    assert_eq!(shapes.len(), 2);
    assert_eq!(doc.update_counts().shapes_live, 1);
}

#[test]
fn a_rect_after_a_path_paints_over_it() {
    // Given: a filled triangle, then a stroked rect, which the icon route
    // would take as a path but not as a rect
    let path = [("d", "M2 2h8v8z"), ("fill", "lime")];
    let rect = [("x", "4"), ("y", "4"), ("width", "6"), ("height", "6")];
    let stroked = [&rect[..], &[("fill", "none"), ("stroke", "currentColor")]].concat();
    let (mut doc, _) = page(&[&[("path", &path), ("rect", &stroked)]]);

    // When
    let shapes = drawn(&mut doc);

    // Then
    let kinds: Vec<_> = shapes.iter().map(|(kind, _)| *kind).collect();
    assert_eq!(kinds, [ShapeKind::Mesh, ShapeKind::ClosedForm]);
    assert_eq!(shapes[0].1, rgb(0, 255, 0));
}

#[test]
fn a_rect_the_icon_route_could_stroke_takes_the_shapes_route() {
    // Given: stroked in its colour with no fill, as an icon path is
    let rect = [
        ("x", "3"),
        ("y", "3"),
        ("width", "10"),
        ("height", "10"),
        ("fill", "none"),
        ("stroke", "currentColor"),
    ];
    let (mut doc, _) = page(&[&[("rect", &rect)]]);

    // When, Then
    let kinds: Vec<_> = drawn(&mut doc).iter().map(|(kind, _)| *kind).collect();
    assert_eq!(kinds, [ShapeKind::ClosedForm]);
}

#[test]
fn a_path_written_as_a_rounded_rect_is_a_closed_form() {
    // Given
    let d = "M5 4H11A2 2 0 0 1 13 6V10A2 2 0 0 1 11 12H5A2 2 0 0 1 3 10V6A2 2 0 0 1 5 4Z";
    let (mut doc, _) = page(&[&[("path", &[("d", d), ("fill", "red")])]]);

    // When, Then
    assert_eq!(drawn(&mut doc), [(ShapeKind::ClosedForm, rgb(255, 0, 0))]);
    assert_eq!(doc.update_counts().shapes_tessellated, 0);
}

#[test]
fn a_pill_and_a_full_ellipse_with_sides_of_no_length_are_closed_forms() {
    // Given: rx half the height (a pill, its short sides 0 long), and
    // both radii half the sides (an ellipse, every side 0 long)
    let pill = [
        ("x", "2"),
        ("y", "6"),
        ("width", "12"),
        ("height", "4"),
        ("rx", "2"),
        ("fill", "red"),
    ];
    let full = [
        ("x", "2"),
        ("y", "4"),
        ("width", "12"),
        ("height", "8"),
        ("rx", "6"),
        ("ry", "4"),
        ("fill", "red"),
    ];
    let (mut doc, _) = page(&[&[("rect", &pill)], &[("rect", &full)]]);

    // When
    let kinds: Vec<_> = drawn(&mut doc).iter().map(|(kind, _)| *kind).collect();

    // Then
    assert_eq!(kinds, [ShapeKind::ClosedForm, ShapeKind::ClosedForm]);
    assert_eq!(doc.update_counts().shapes_tessellated, 0);
}

#[test]
fn a_rect_whose_width_becomes_zero_draws_nothing_and_is_not_refused() {
    // Given: a progress bar, drawn at width 6 (#391 review, M1)
    let bar = [
        ("x", "2"),
        ("y", "6"),
        ("width", "6"),
        ("height", "4"),
        ("fill", "red"),
    ];
    let (mut doc, svgs) = page(&[&[("rect", &bar)]]);
    assert_eq!(drawn(&mut doc).len(), 1);
    let rect = doc.children(svgs[0])[0];

    // When: its width reaches 0, then grows again
    doc.set_attribute(rect, "width", "0");
    let at_zero = drawn(&mut doc);
    doc.set_attribute(rect, "width", "3");

    // Then
    assert_eq!(at_zero, []);
    assert_eq!(drawn(&mut doc).len(), 1);
}

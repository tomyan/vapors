//! `<rect>` and `<circle>` in an svg (one-scene S4b): drawn on the
//! shapes route as closed forms, never tessellated. One cache slot is
//! held for each distinct shape. A rect paints in document order with
//! paths. What they refuse is refused naming the attribute, and a
//! change to one alone rechecks its svg.

mod support;

use support::svg_page::{SVG, assert_refused, changed, with_attributes};
use support::{document, element, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;
use vapors_scene::{Color, Item, ShapeKind};

/// An element's attributes, name and value.
type Attributes<'a> = &'a [(&'a str, &'a str)];
/// A shape element: its tag and attributes.
type Shape<'a> = (&'a str, Attributes<'a>);

const RECT: &[(&str, &str)] = &[
    ("x", "2"),
    ("y", "3"),
    ("width", "12"),
    ("height", "8"),
    ("rx", "3"),
    ("ry", "1.5"),
    ("fill", "red"),
];
const CIRCLE: &[(&str, &str)] = &[("cx", "8"), ("cy", "8"), ("r", "5"), ("fill", "blue")];

/// A body holding one svg per entry of `svgs`, each holding a shape
/// element per `(tag, attributes)`; the svgs.
fn page(svgs: &[&[Shape]]) -> (Document, Vec<NodeId>) {
    let mut doc = document();
    let body = doc.body();
    let nodes = (svgs.iter())
        .map(|shapes| {
            let svg = element(&mut doc, body, "svg", "icon");
            with_attributes(&mut doc, svg, SVG);
            for (tag, attributes) in *shapes {
                let shape = element(&mut doc, svg, tag, "");
                with_attributes(&mut doc, shape, attributes);
            }
            svg
        })
        .collect();
    doc.add_stylesheet("app.css", ".icon { display: block; }")
        .expect("parses");
    (doc, nodes)
}

/// Each shape item's kind and colour, in paint order.
fn drawn(doc: &mut Document) -> Vec<(ShapeKind, Color)> {
    let painted = doc.update(viewport(400.0, 300.0)).expect("updates");
    (painted.scene().to_scene().items().iter())
        .filter_map(|item| match item {
            Item::Shape(shape) => Some((shape.kind(), shape.colour)),
            _ => None,
        })
        .collect()
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba8(r, g, b, 255)
}

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
fn a_rect_or_circle_refuses_what_svg_would_read_otherwise() {
    // Given: a unit; a percentage; a transform; a missing radius
    let cases: [(&str, Attributes, &[&str]); 4] = [
        (
            "rect",
            &[("width", "12px")],
            &[
                "<rect>",
                "width \"12px\" is not a number above 0 with no unit",
            ],
        ),
        (
            "rect",
            &[("x", "10%")],
            &["<rect>", "x \"10%\" is not a number with no unit"],
        ),
        (
            "rect",
            &[("transform", "scale(2)")],
            &["its transform attribute is not in the icon subset"],
        ),
        (
            "circle",
            &[("r", "")],
            &["<circle>", "its r attribute is required"],
        ),
    ];

    for (tag, change, says) in cases {
        let base = if tag == "rect" { RECT } else { CIRCLE };
        let (mut doc, _) = page(&[&[(tag, &changed(base, change))]]);

        // When, Then
        assert_refused(&mut doc, says);
    }
}

#[test]
fn a_rect_changed_alone_to_reach_outside_the_view_box_is_refused() {
    // Given: updated once
    let (mut doc, svgs) = page(&[&[("rect", RECT)]]);
    drawn(&mut doc);
    let rect = doc.children(svgs[0])[0];

    // When: only the rect changes
    doc.set_attribute(rect, "width", "20");

    // Then: refused, naming it
    assert_refused(&mut doc, &["its rect 1 draws from", "outside its viewBox"]);
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

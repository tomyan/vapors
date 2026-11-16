//! Which route draws an svg's paths (one-scene S4a, `svg_route.rs`):
//! the icon route while every path is in the icon subset; shapes, for
//! the whole svg, once a path has a fill or a stroke in another colour;
//! a path that paints nothing, or one that reaches outside the view
//! box, refused naming it. A class change that only recolours
//! tessellates nothing.

use crate::support;

use support::svg_page::{assert_refused, icon};
use support::viewport;
use vapors_dom::Dom;
use vapors_elements::{Document, Painted};
use vapors_scene::{Color, Item, ShapeKind};

fn updated(doc: &mut Document) -> &Painted {
    doc.update(viewport(400.0, 300.0)).expect("updates")
}

/// Each item that draws the svg: `I` for its icon, or each shape's
/// kind and colour.
fn drawn(painted: &Painted) -> (usize, Vec<(ShapeKind, Color)>) {
    let scene = painted.scene().to_scene();
    let icons = scene
        .items()
        .iter()
        .filter(|item| matches!(item, Item::Icon { .. }))
        .count();
    let shapes = scene
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Shape(shape) => Some((shape.kind(), shape.colour)),
            _ => None,
        })
        .collect();
    (icons, shapes)
}

fn red() -> Color {
    Color::from_rgba8(255, 0, 0, 255)
}

fn blue() -> Color {
    Color::from_rgba8(0, 0, 255, 255)
}

fn black() -> Color {
    Color::from_rgba8(0, 0, 0, 255)
}

#[test]
fn a_path_in_the_icon_subset_takes_the_icon_route() {
    // Given: send, stroked in currentColor
    let (mut doc, _) = icon("", &[], &[]);

    // When, Then
    assert_eq!(drawn(updated(&mut doc)), (1, vec![]));
}

#[test]
fn a_filled_path_takes_the_shapes_route() {
    // Given
    let (mut doc, _) = icon("", &[], &[("fill", "red"), ("stroke", "")]);

    // When, Then
    assert_eq!(
        drawn(updated(&mut doc)),
        (0, vec![(ShapeKind::Mesh, red())])
    );
}

#[test]
fn a_path_filled_and_stroked_paints_its_fill_then_its_stroke() {
    // Given: a rect path, filled red and stroked blue
    let (mut doc, _) = icon(
        "",
        &[],
        &[("d", "M3 4h10v8h-10z"), ("fill", "red"), ("stroke", "blue")],
    );

    // When, Then: the rect is a closed form both times
    assert_eq!(
        drawn(updated(&mut doc)),
        (
            0,
            vec![
                (ShapeKind::ClosedForm, red()),
                (ShapeKind::ClosedForm, blue())
            ]
        )
    );
}

#[test]
fn a_stroke_in_another_colour_takes_the_shapes_route() {
    // Given
    let (mut doc, _) = icon("", &[], &[("stroke", "blue")]);

    // When, Then
    assert_eq!(
        drawn(updated(&mut doc)),
        (0, vec![(ShapeKind::Mesh, blue())])
    );
}

#[test]
fn one_filled_path_takes_its_whole_svg_to_shapes() {
    // Given: send stroked in currentColor (black), and a filled path
    let (mut doc, svg) = icon("", &[], &[]);
    let path = doc.create_element("path");
    for (name, value) in [("d", "M2 2h4v4z"), ("fill", "red")] {
        doc.set_attribute(path, name, value);
    }
    doc.append(svg, path);

    // When, Then: both paths as shapes, in order
    assert_eq!(
        drawn(updated(&mut doc)),
        (
            0,
            vec![(ShapeKind::Mesh, black()), (ShapeKind::Mesh, red())]
        )
    );
}

#[test]
fn a_path_reaching_outside_the_view_box_is_refused_naming_it() {
    // Given: a fill from x = 2 to 18 in a 16-unit view box
    let (mut doc, _) = icon("", &[], &[("d", "M2 2h16v4z"), ("fill", "red")]);

    // When, Then
    assert_refused(
        &mut doc,
        &[
            "its path 1 draws from (2, 2) to (18, 6)",
            "outside its viewBox \"0 0 16 16\"",
            "drawn as shapes because path 1 has a fill",
            "not clipped yet",
        ],
    );
}

#[test]
fn a_class_that_recolours_a_fill_tessellates_nothing() {
    // Given: painted once, its fill and its stroke tessellated
    let (mut doc, svg) = icon("} .icon.blue path { fill: blue", &[], &[("fill", "red")]);
    updated(&mut doc).shape_changes_taken();
    assert_eq!(doc.update_counts().shapes_tessellated, 2);

    // When
    doc.set_attribute(svg, "class", "icon blue");
    let painted = updated(&mut doc);

    // Then: no new shape for the painter's store to take, so it
    // uploads nothing
    assert_eq!(painted.shape_changes().1.len(), 0);
    let found = drawn(painted);
    assert_eq!(
        found,
        (
            0,
            vec![
                (ShapeKind::Mesh, blue()),
                (ShapeKind::Mesh, Color::from_rgba8(0, 0, 0, 255))
            ]
        )
    );
    assert_eq!(doc.update_counts().shapes_tessellated, 0);
}

#[test]
fn an_svg_back_on_the_icon_route_frees_its_shapes() {
    // Given: send filled red, so on shapes, live in the cache
    let (mut doc, svg) = icon("} .icon.plain path { fill: none", &[], &[("fill", "red")]);
    updated(&mut doc);
    assert!(doc.update_counts().shapes_live > 0);

    // When: a class takes its fill away, so the icon route draws it
    doc.set_attribute(svg, "class", "icon plain");
    let painted = updated(&mut doc);

    // Then
    assert_eq!(drawn(painted).0, 1, "the icon");
    assert_eq!(doc.update_counts().shapes_live, 0);
}

#[test]
fn a_removed_svg_frees_its_shapes() {
    // Given
    let (mut doc, svg) = icon("", &[], &[("fill", "red")]);
    updated(&mut doc);

    // When
    doc.remove(svg);
    updated(&mut doc);

    // Then
    assert_eq!(doc.update_counts().shapes_live, 0);
}

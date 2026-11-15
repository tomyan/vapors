//! What an svg on the shapes route refuses, and when it frees its
//! shapes (S4a PR 3, #381 review): a change to a path alone rechecks
//! its svg, so a path reaching outside the view box or one the cache
//! cannot make refuses the update before paint; a hidden svg releases
//! its shapes; the icon atlas's size cap is the icon route's alone; a
//! circle path is a closed form.

mod support;

use support::svg_page::{assert_refused, icon};
use support::viewport;
use vapors_dom::Dom;
use vapors_elements::{Document, Painted};
use vapors_scene::{Color, Item, ShapeKind};

fn updated(doc: &mut Document) -> &Painted {
    doc.update(viewport(400.0, 300.0)).expect("updates")
}

fn red() -> Color {
    Color::from_rgba8(255, 0, 0, 255)
}

/// The svg's icon items, and each shape's kind and colour.
fn drawn(painted: &Painted) -> (usize, Vec<(ShapeKind, Color)>) {
    let scene = painted.scene().to_scene();
    let icons = (scene.items().iter())
        .filter(|item| matches!(item, Item::Icon { .. }))
        .count();
    let shapes = (scene.items().iter())
        .filter_map(|item| match item {
            Item::Shape(shape) => Some((shape.kind(), shape.colour)),
            _ => None,
        })
        .collect();
    (icons, shapes)
}

#[test]
fn a_path_changed_after_the_first_update_to_reach_outside_is_refused() {
    // Given: a filled path inside the view box, updated once
    let (mut doc, svg) = icon("", &[], &[("d", "M2 2h4v4z"), ("fill", "red")]);
    updated(&mut doc);
    let path = doc.children(svg)[0];

    // When: only the path changes
    doc.set_attribute(path, "d", "M2 2h16v4z");

    // Then: refused; and put back inside, it paints again
    assert_refused(&mut doc, &["outside its viewBox", "not clipped yet"]);
    doc.set_attribute(path, "d", "M3 3h8v8h-8z");
    assert_eq!(drawn(updated(&mut doc)).1.len(), 2);
}

#[test]
fn a_path_changed_after_the_first_update_to_one_the_cache_refuses_is_refused() {
    // Given
    let (mut doc, svg) = icon("", &[], &[("d", "M2 2h4v4z"), ("fill", "red")]);
    updated(&mut doc);
    let path = doc.children(svg)[0];

    // When: a fill of no area, which the tessellator refuses
    doc.set_attribute(path, "d", "M2 2L2 2");

    // Then: refused before paint, so paint does not panic
    assert_refused(&mut doc, &["a path cannot be drawn", "path 1"]);
}

#[test]
fn a_path_drawn_past_the_largest_scale_is_refused() {
    // Given: a view box a thousandth of a unit wide in a 16 px svg
    let (mut doc, _) = icon(
        "",
        &[("viewBox", "0 0 0.001 0.001")],
        &[("d", "M0 0h0.0005v0.0005z"), ("fill", "red")],
    );

    // When, Then
    assert_refused(&mut doc, &["a path cannot be drawn", "past the 4096"]);
}

#[test]
fn a_hidden_svg_releases_its_shapes() {
    for css in [
        "} .icon.gone { visibility: hidden",
        "} .icon.gone { display: none",
    ] {
        // Given: on shapes, live
        let (mut doc, svg) = icon(css, &[], &[("fill", "red")]);
        updated(&mut doc);
        assert!(doc.update_counts().shapes_live > 0, "{css}");

        // When
        doc.set_attribute(svg, "class", "icon gone");
        updated(&mut doc);

        // Then
        assert_eq!(doc.update_counts().shapes_live, 0, "{css}");
    }
}

#[test]
fn an_svg_on_shapes_is_not_held_to_the_icon_atlass_size() {
    // Given: 300 px, past the atlas's 256, filled
    let (mut doc, _) = icon(
        "",
        &[("width", "300"), ("height", "300")],
        &[("fill", "red")],
    );

    // When, Then
    assert_eq!(drawn(updated(&mut doc)).1.len(), 2);
}

#[test]
fn a_circle_path_is_a_closed_form() {
    // Given
    let (mut doc, _) = icon(
        "",
        &[],
        &[
            ("d", "M2 8a6 6 0 1 0 12 0a6 6 0 1 0-12 0z"),
            ("fill", "red"),
            ("stroke", ""),
        ],
    );

    // When, Then
    assert_eq!(
        drawn(updated(&mut doc)),
        (0, vec![(ShapeKind::ClosedForm, red())])
    );
}

#[test]
fn a_grown_svg_whose_width_changes_without_a_change_to_it_is_checked_again() {
    // Given: a filled svg growing in a 300 px flex row beside a 100 px
    // div, so 200 px wide
    let (mut doc, _) = icon(
        "flex-grow: 1; } body { display: flex; width: 300px; } .side { width: 100px; } .side.wide { width: 290px; } .x {",
        &[],
        &[("fill", "red")],
    );
    let side = doc.create_element("div");
    doc.set_attribute(side, "class", "side");
    let body = doc.body();
    doc.append(body, side);
    updated(&mut doc);

    // When: only the sibling changes, which shrinks the svg to 10 px,
    // narrower than its 16 px height, so its scale falls
    doc.set_attribute(side, "class", "side wide");
    let painted = updated(&mut doc);

    // Then: drawn at its new size, its shapes checked at it first (paint
    // asserts so)
    assert_eq!(drawn(painted).1.len(), 2);
}

#[test]
fn an_svg_under_an_ancestor_made_display_none_releases_its_shapes() {
    // Given: a filled svg in the body, live
    let (mut doc, _) = icon(
        "} body.gone { display: none; } .x {",
        &[],
        &[("fill", "red")],
    );
    updated(&mut doc);
    assert!(doc.update_counts().shapes_live > 0);

    // When: only the body changes; the svg is not restyled
    let body = doc.body();
    doc.set_attribute(body, "class", "gone");
    updated(&mut doc);

    // Then: it has no box after layout, so its shapes are released
    assert_eq!(doc.update_counts().shapes_live, 0);
}

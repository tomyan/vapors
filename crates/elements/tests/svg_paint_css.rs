//! An icon's paint from the cascade (S4a PR 1): `fill`, `stroke` and
//! `stroke-width` are CSS properties, the path's attributes enter as
//! presentation attributes below every rule, as Chromium ranks them,
//! and a change after the first paint rebuilds the icon.

use crate::support;

use support::svg_page::{assert_accepted, icon};
use vapors_dom::Dom;
use vapors_elements::{Document, Painted};
use vapors_scene::Item;
use vapors_shapes::IconPath;
use vapors_text::{Icon, IconStroke};

/// The export icon of `svg_page`, its one path stroked `width` wide.
fn export(width: f32) -> Icon {
    let stroke = IconStroke {
        path: IconPath::parse("M2 8h11M9 4l4 4-4 4").expect("parses"),
        width,
    };
    Icon::new([0.0, 0.0, 16.0, 16.0], vec![stroke]).expect("a valid icon")
}

/// Whether the one icon `painted` shows is the export icon stroked
/// `width` wide.
fn stroked(painted: &Painted, width: f32) -> bool {
    let scene = painted.scene().to_scene();
    let id = scene
        .items()
        .iter()
        .find_map(|item| match *item {
            Item::Icon { icon, .. } => Some(icon),
            _ => None,
        })
        .expect("an icon item");
    **painted.icon(id) == export(width)
}

fn updated(doc: &mut Document) -> &Painted {
    doc.update(support::viewport(400.0, 300.0))
        .expect("updates")
}

#[test]
fn a_rule_beats_the_paths_stroke_width_attribute() {
    // Given: stroke-width="1.5" on the path
    let (mut doc, _) = icon("} .icon path { stroke-width: 2.5", &[], &[]);

    // When, Then
    assert!(stroked(updated(&mut doc), 2.5));
}

#[test]
fn the_svgs_stroke_width_attribute_reaches_a_path_without_one() {
    // Given
    let (mut doc, _) = icon("", &[("stroke-width", "2")], &[("stroke-width", "")]);

    // When, Then
    assert!(stroked(updated(&mut doc), 2.0));
}

#[test]
fn a_class_on_the_svg_that_widens_its_paths_rebuilds_the_icon() {
    // Given: painted once at the attribute's 1.5
    let (mut doc, svg) = icon("} .icon.bold path { stroke-width: 3", &[], &[]);
    assert!(stroked(updated(&mut doc), 1.5));

    // When
    doc.set_attribute(svg, "class", "icon bold");

    // Then
    assert!(stroked(updated(&mut doc), 3.0));
}

#[test]
fn a_stroke_width_in_px_is_accepted_as_chromium_accepts_it() {
    // Given
    let (mut doc, _) = icon("", &[], &[("stroke-width", "1px")]);

    // When, Then
    assert_accepted(&mut doc);
    assert!(stroked(updated(&mut doc), 1.0));
}

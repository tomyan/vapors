//! A range input's paint (3-E3a): a track across its content box and a
//! round thumb at the value, from the `--vapors-*` parts, with the
//! user-agent parts when none are set; nothing when hidden.

mod support;

use support::{document, element, viewport};
use vapors_dom::Dom;
use vapors_elements::Viewport;
use vapors_scene::{Color, Item, Quad, Rect};

const SHEET: &str = "body { margin: 0; } input { display: block; box-sizing: border-box; width: 200px; height: 20px; margin: 0; padding: 0; }";

/// A document with one range, 200 x 20 at the top left, at 30 of 0..100.
fn range(style: &str) -> vapors_elements::Document {
    let mut doc = document();
    let body = doc.body();
    let input = element(&mut doc, body, "input", "");
    doc.set_attribute(input, "type", "range");
    doc.set_attribute(input, "value", "30");
    doc.add_stylesheet("app.css", &format!("{SHEET} {style}"))
        .expect("parses");
    doc
}

fn quads(doc: &mut vapors_elements::Document, scale: f32) -> Vec<Quad> {
    let painted = doc
        .update(Viewport {
            width: 400.0,
            height: 100.0,
            scale,
        })
        .expect("updates");
    painted
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Quad(quad) => Some(*quad),
            _ => None,
        })
        .collect()
}

#[test]
fn the_user_agent_track_and_thumb_are_drawn_at_the_value() {
    // Given
    let mut doc = range("");

    // When
    let quads = quads(&mut doc, 1.0);

    // Then: a 4 px track across the middle, a 16 px thumb centred at
    // 8 + 0.3 * 184 = 63.2, so x 55.2..71.2, snapped to 55..71
    let [track, thumb] = &quads[..] else {
        panic!("a track and a thumb, got {quads:?}");
    };
    assert_eq!(track.rect, Rect::new(0.0, 8.0, 200.0, 12.0));
    assert_eq!(track.radii, [2.0; 4]);
    assert_eq!(thumb.rect, Rect::new(55.0, 2.0, 71.0, 18.0));
    assert_eq!(thumb.radii, [8.0; 4]);
    assert_eq!(thumb.background, Color::from_rgba8(0, 96, 223, 255));
}

#[test]
fn the_parts_properties_set_the_sizes_and_colours_inheriting() {
    // Given: tokens on the body
    let mut doc = range(
        "body { --vapors-thumb-size: 10px; --vapors-thumb-color: #ff0000; --vapors-track-height: 2px; --vapors-track-color: rgb(0 0 255) }",
    );

    // When: at 2x
    let quads = quads(&mut doc, 2.0);

    // Then: the thumb centred at 5 + 0.3 * 190 = 62, x 57..67, y 5..15,
    // doubled
    let [track, thumb] = &quads[..] else {
        panic!("a track and a thumb, got {quads:?}");
    };
    assert_eq!(track.rect, Rect::new(0.0, 18.0, 400.0, 22.0));
    assert_eq!(track.background, Color::from_rgba8(0, 0, 255, 255));
    assert_eq!(thumb.rect, Rect::new(114.0, 10.0, 134.0, 30.0));
    assert_eq!(thumb.background, Color::from_rgba8(255, 0, 0, 255));
    assert_eq!(thumb.radii, [10.0; 4]);
}

#[test]
fn a_hidden_range_paints_no_parts() {
    // Given
    let mut doc = range("input { visibility: hidden; }");

    // When
    let quads = quads(&mut doc, 1.0);

    // Then
    assert!(quads.is_empty(), "{quads:?}");
}

#[test]
fn a_range_without_a_size_is_chromiums_129_by_16() {
    // Given: no size of its own, in a flex row as the corpus lays it out
    let mut doc = document();
    let body = doc.body();
    let input = element(&mut doc, body, "input", "");
    doc.set_attribute(input, "type", "range");
    doc.add_stylesheet("app.css", "body { margin: 0; display: flex; }")
        .expect("parses");

    // When
    let painted = doc.update(viewport(400.0, 100.0)).expect("updates");

    // Then: with the user-agent 2 px margin
    let border_box = painted.border_box(input).expect("a box");
    assert_eq!(
        (
            border_box.x,
            border_box.y,
            border_box.width,
            border_box.height
        ),
        (2.0, 2.0, 129.0, 16.0)
    );
}

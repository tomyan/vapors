//! A checkbox's box and mark (3-E3b): Chromium's 13 x 13 px with its
//! margins, stretched across a flex line as Chromium's is; a mark in
//! `--vapors-mark-color` when checked, inset a quarter of the box.

mod support;

use support::{document, element, viewport};
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_scene::{Color, Item, Rect};

/// A document with one checkbox in a flex row of `height`.
fn row(height: &str, checked: bool, style: &str) -> (Document, vapors_dom::NodeId) {
    let mut doc = document();
    let body = doc.body();
    let input = element(&mut doc, body, "input", "");
    doc.set_attribute(input, "type", "checkbox");
    if checked {
        doc.set_attribute(input, "checked", "");
    }
    doc.add_stylesheet(
        "app.css",
        &format!("body {{ margin: 0; display: flex; height: {height}; }} {style}"),
    )
    .expect("parses");
    (doc, input)
}

#[test]
fn a_checkbox_without_a_size_is_chromiums_13_by_13_with_its_margins() {
    // Given: a row as tall as the checkbox, so nothing stretches
    let (mut doc, input) = row("19px", false, "");

    // When
    let painted = doc.update(viewport(100.0, 100.0)).expect("updates");

    // Then: the natural size and the UA border make 13; margins 3 3 3 4
    let r = painted.border_box(input).expect("a box");
    assert_eq!((r.x, r.y, r.width, r.height), (4.0, 3.0, 13.0, 13.0));
}

#[test]
fn a_checkbox_stretches_across_a_flex_line_as_chromiums_does() {
    // Given: a 40 px row (`fixtures/checkbox.boxes.json`)
    let (mut doc, input) = row("40px", false, "");

    // When
    let painted = doc.update(viewport(100.0, 100.0)).expect("updates");

    // Then
    let r = painted.border_box(input).expect("a box");
    assert_eq!((r.width, r.height), (13.0, 34.0));
}

#[test]
fn a_checked_box_draws_its_mark_inset_a_quarter_and_an_unchecked_none() {
    // Given: a 20 px box (content 18), the mark colour from a token
    let style =
        "input { margin: 0; width: 20px; height: 20px; } body { --vapors-mark-color: #00ff00; }";
    let (mut on, _) = row("20px", true, style);
    let (mut off, _) = row("20px", false, style);

    // When
    let marks = |doc: &mut Document| {
        let painted = doc.update(viewport(100.0, 100.0)).expect("updates");
        painted
            .scene()
            .items()
            .iter()
            .filter_map(|item| match item {
                Item::Quad(q) if q.background == Color::from_rgba8(0, 255, 0, 255) => {
                    Some((q.rect, q.radii))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };

    // Then: content 1..19, inset 4.5: 5.5..14.5, snapped to 6..15 (14.5
    // rounds to 14 in f32's round-half-away: 15)
    assert_eq!(
        marks(&mut on),
        [(Rect::new(6.0, 6.0, 15.0, 15.0), [1.0; 4])]
    );
    assert_eq!(marks(&mut off), []);
}

//! An element's `style` attribute: applied over the sheets, refused
//! naming the element when it does not parse, and named as the
//! attribute when one of its values is refused.

mod support;

use support::{document, element, refusals, viewport};
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_scene::{Color, Item};

/// The background of every quad painted, in paint order.
fn fills(doc: &mut Document) -> Vec<Color> {
    let painted = doc.update(viewport(100.0, 100.0)).expect("updates");
    painted
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Quad(quad) => Some(quad.background),
            _ => None,
        })
        .collect()
}

#[test]
fn a_style_attribute_is_applied_over_the_sheets() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let div = element(&mut doc, body, "div", "panel");
    doc.add_stylesheet(
        "app.css",
        ".panel { height: 2px; background-color: #ff0000; }",
    )
    .expect("parses");
    doc.set_attribute(div, "style", "background-color: #0000ff");

    // When
    let fills = fills(&mut doc);

    // Then
    assert_eq!(fills, [Color::from_rgba8(0, 0, 255, 255)]);
}

#[test]
fn a_changed_style_attribute_is_applied_at_the_next_update() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let div = element(&mut doc, body, "div", "");
    doc.set_attribute(div, "style", "height: 2px; background-color: #0000ff");
    fills(&mut doc);
    doc.take_changed();

    // When
    doc.set_attribute(div, "style", "height: 2px; background-color: #00ff00");

    // Then
    assert!(doc.take_changed(), "the change asks for a frame");
    assert_eq!(fills(&mut doc), [Color::from_rgba8(0, 255, 0, 255)]);
}

#[test]
fn a_style_attribute_that_does_not_parse_is_refused_naming_the_element() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let div = element(&mut doc, body, "div", "panel");
    doc.set_attribute(div, "style", "colour: red");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with("<div class=\"panel\">: its style attribute 1:1:")
            && errors[0].contains("colour"),
        "{errors:?}"
    );
}

#[test]
fn a_refused_value_from_a_style_attribute_names_the_attribute() {
    // Given: opacity is not honoured yet
    let mut doc = document();
    let body = doc.body();
    let div = element(&mut doc, body, "div", "card");
    doc.set_attribute(div, "style", "height: 2px; opacity: 0.5");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with("style attribute:1:14: <div class=\"card\">: `opacity: 0.5`"),
        "{errors:?}"
    );
}

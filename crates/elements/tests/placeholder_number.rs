//! The placeholder's text against the field's attributes
//! (`docs/placeholder.md`): a number field paints it, a `value` hides
//! it, line breaks in it are removed.

use crate::support;

use support::{document, element, viewport};
use vapors_dom::Dom;
use vapors_scene::{Color, Item};

/// The strings and colours a 100 px input with `attributes` paints,
/// with `css` added to its rule, leaving out an empty editor's line.
fn painted(attributes: &[(&str, &str)], css: &str) -> Vec<(String, Color)> {
    let mut doc = document();
    let body = doc.body();
    let input = element(&mut doc, body, "input", "");
    for (name, value) in attributes {
        doc.set_attribute(input, name, value);
    }
    let sheet = format!("input {{ display: block; width: 100px; {css} }}");
    doc.add_stylesheet("a.css", &sheet).expect("parses");
    let painted = doc.update(viewport(200.0, 100.0)).expect("updates");
    let scene = painted.scene().to_scene();
    let mut found = Vec::new();
    for item in scene.items() {
        if let Item::Text { text, .. } = item {
            let layout = painted.text(*text);
            // an empty field's editor line is one no-break space
            if layout.text() != "\u{a0}" {
                found.push((layout.text().to_owned(), layout.runs()[0].color));
            }
        }
    }
    found
}

#[test]
fn an_empty_number_field_paints_its_placeholder_in_grey() {
    // Given, when
    let found = painted(&[("type", "number"), ("placeholder", "Qty")], "");

    // Then
    assert_eq!(
        found,
        [("Qty".into(), Color::from_rgba8(117, 117, 117, 255))]
    );
}

#[test]
fn a_number_field_with_a_value_paints_none() {
    // Given, when
    let found = painted(
        &[("type", "number"), ("placeholder", "Qty"), ("value", "3")],
        "",
    );

    // Then
    assert!(found.iter().all(|t| t.0 != "Qty"), "{found:?}");
}

#[test]
fn a_placeholder_of_only_line_breaks_paints_nothing() {
    // Given, when: Chromium strips the breaks and shows an empty one
    let found = painted(&[("placeholder", "\r\n\n")], "");

    // Then
    assert_eq!(found, []);
}

#[test]
fn a_text_field_with_a_value_attribute_paints_none() {
    // Given, when
    let found = painted(&[("placeholder", "Search"), ("value", "ada")], "");

    // Then
    assert_eq!(
        found.iter().map(|t| t.0.as_str()).collect::<Vec<_>>(),
        ["ada"]
    );
}

#[test]
fn line_breaks_inside_a_placeholder_are_removed_not_replaced() {
    // Given, when
    let found = painted(&[("placeholder", "Sea\r\nrch")], "");

    // Then
    assert_eq!(
        found.iter().map(|t| t.0.as_str()).collect::<Vec<_>>(),
        ["Search"]
    );
}

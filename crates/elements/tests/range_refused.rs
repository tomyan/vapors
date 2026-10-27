//! What a range input refuses (3-E3a): an input of a type not built
//! yet, naming its slice; a part's custom property of another shape,
//! named where it is declared; `disabled="false"`; a boolean `value`.

mod support;

use support::{element, refusals};
use vapors_dom::{Dom, PropertyValue};

/// A document whose inputs are blocks, so the only refusal is the test's.
fn document() -> vapors_elements::Document {
    let mut doc = support::document();
    doc.add_stylesheet("blocks.css", "input { display: block; }")
        .expect("parses");
    doc
}

#[test]
fn an_input_of_a_type_not_built_yet_is_refused_naming_its_slice() {
    for (kind, slice) in [
        (Some("checkbox"), "3-E3b"),
        (Some("number"), "3-E3c"),
        (Some("text"), "3-E6"),
        (None, "3-E6"),
        (Some("date"), "no slice yet"),
    ] {
        // Given
        let mut doc = document();
        let body = doc.body();
        let input = element(&mut doc, body, "input", "");
        if let Some(kind) = kind {
            doc.set_attribute(input, "type", kind);
        }

        // When
        let errors = refusals(&mut doc);

        // Then
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("<input type=") && errors[0].contains(slice),
            "{kind:?}: {errors:?}"
        );
    }
}

#[test]
fn a_parts_property_of_another_shape_is_refused_where_it_is_declared() {
    for (declared, needle) in [
        (
            "--vapors-thumb-size: red",
            "`--vapors-thumb-size: red` is not a length in px",
        ),
        ("--vapors-thumb-size: 2em", "is not a length in px"),
        ("--vapors-track-height: -1px", "is not a length in px"),
        (
            "--vapors-track-color: 4px",
            "`--vapors-track-color: 4px` is not a colour",
        ),
        ("--vapors-thumb-color: currentColor", "is not a colour"),
    ] {
        // Given: declared on the body, inherited by the range
        let mut doc = document();
        let body = doc.body();
        let input = element(&mut doc, body, "input", "");
        doc.set_attribute(input, "type", "range");
        doc.add_stylesheet("app.css", &format!("body {{ {declared} }}"))
            .expect("parses");

        // When
        let errors = refusals(&mut doc);

        // Then: once, at the body, not again at the range
        assert_eq!(errors.len(), 1, "{declared}: {errors:?}");
        assert!(
            errors[0].starts_with("<body>") && errors[0].contains(needle),
            "{declared}: {errors:?}"
        );
    }
}

#[test]
fn a_disabled_false_on_an_input_is_refused_as_on_a_button() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let input = element(&mut doc, body, "input", "");
    doc.set_attribute(input, "type", "range");
    doc.set_attribute(input, "disabled", "false");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("disabled=\"false\""), "{errors:?}");
}

#[test]
fn a_non_finite_value_property_is_refused_naming_the_signal() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        // Given
        let mut doc = document();
        let body = doc.body();
        let input = element(&mut doc, body, "input", "");
        doc.set_attribute(input, "type", "range");
        doc.set_property(input, "value", PropertyValue::Number(bad));

        // When
        let errors = refusals(&mut doc);

        // Then
        assert_eq!(errors.len(), 1, "{bad}: {errors:?}");
        assert!(
            errors[0].contains("not a number an input can hold") && errors[0].contains("signal"),
            "{bad}: {errors:?}"
        );
    }
}

#[test]
fn a_boolean_value_property_is_refused() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let input = element(&mut doc, body, "input", "");
    doc.set_attribute(input, "type", "range");
    doc.set_property(input, "value", PropertyValue::Bool(true));

    // When
    let errors = refusals(&mut doc);

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("<input>") && errors[0].contains("boolean"),
        "{errors:?}"
    );
}

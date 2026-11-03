//! What a number field shows: a value set as a number, at its step's
//! precision (touché, ux-review-1: "7.1465" with step 1); the value
//! itself and typed text unchanged. Chromium shows the full number;
//! this differs on purpose, for display only.

use std::collections::BTreeMap;

use vapors_dom::PropertyValue;

use super::{Field, shown};

fn with(
    step: &str,
    value: PropertyValue,
) -> (BTreeMap<String, String>, BTreeMap<String, PropertyValue>) {
    let attributes = BTreeMap::from([("step".to_owned(), step.to_owned())]);
    let properties = BTreeMap::from([("value".to_owned(), value)]);
    (attributes, properties)
}

#[test]
fn a_number_set_on_a_field_shows_at_its_steps_precision() {
    for (step, value, expected) in [
        ("1", 7.1465, "7"),
        ("0.05", 0.123_456, "0.12"),
        ("0.1", -2.26, "-2.3"),
        ("1", -0.2, "0"),
        ("2.5", 5.0, "5.0"),
    ] {
        let (a, p) = with(step, PropertyValue::Number(value));
        assert_eq!(shown(&a, &p), expected, "step {step}, {value}");
    }
}

#[test]
fn with_step_any_or_typed_text_the_field_shows_it_all() {
    let (a, p) = with("any", PropertyValue::Number(0.123_456));
    assert_eq!(shown(&a, &p), "0.123456");
    let (a, p) = with("1", PropertyValue::Text("7.1465".to_owned()));
    assert_eq!(shown(&a, &p), "7.1465");
}

#[test]
fn the_fields_value_keeps_the_full_number() {
    let (a, p) = with("1", PropertyValue::Number(7.1465));
    assert_eq!(Field::of(&a, &p).value(), "7.1465");
}

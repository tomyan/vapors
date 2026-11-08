//! What a number field shows: a value set as a number, at its step's
//! precision (touché, ux-review-1: "7.1465" with step 1); the value
//! itself and typed text unchanged. Chromium shows the full number;
//! this differs on purpose, for display only (`number_shown.rs`).

use std::collections::BTreeMap;

use vapors_dom::PropertyValue;

use crate::number::Field;

fn field(attributes: &[(&str, &str)], value: PropertyValue) -> Field {
    let attributes = attributes
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    let properties = BTreeMap::from([("value".to_owned(), value)]);
    Field::of(&attributes, &properties)
}

fn shown(attributes: &[(&str, &str)], value: f64) -> String {
    field(attributes, PropertyValue::Number(value)).shown
}

#[test]
fn a_number_set_on_a_field_shows_at_its_steps_precision() {
    for (step, value, expected) in [
        ("1", 7.1465, "7"),
        ("0.05", 0.123_456, "0.12"),
        ("0.1", -2.26, "-2.3"),
        ("2.5", 5.0, "5.0"),
        ("0.01", 0.500_000_1, "0.50"),
        ("0.000001", 0.123_456_7, "0.123457"),
    ] {
        assert_eq!(shown(&[("step", step)], value), expected, "step {step}");
    }
}

#[test]
fn a_shown_value_is_never_negative_zero() {
    for (step, value, expected) in [("1", -0.2, "0"), ("1", -0.0, "0"), ("0.01", -0.004, "0.00")] {
        assert_eq!(shown(&[("step", step)], value), expected, "{value}");
    }
}

#[test]
fn halves_round_away_from_zero_as_javascripts_to_fixed() {
    for (step, value, expected) in [("1", 2.5, "3"), ("1", -2.5, "-3"), ("0.01", 0.125, "0.13")] {
        assert_eq!(shown(&[("step", step)], value), expected, "{value}");
    }
}

#[test]
fn a_huge_value_shows_every_digit_with_no_exponent() {
    assert_eq!(shown(&[("step", "1")], 1e21), "1000000000000000000000");
    assert!(shown(&[("step", "0.01")], 1e307).ends_with(".00"));
}

#[test]
fn a_value_outside_the_range_shows_in_full_so_its_invalid_style_has_a_reason() {
    assert_eq!(shown(&[("step", "1"), ("max", "7")], 7.4), "7.4");
    assert_eq!(shown(&[("step", "1"), ("min", "7")], 6.6), "6.6");
}

#[test]
fn without_a_usable_step_attribute_the_value_shows_in_full() {
    for step in [None, Some("any"), Some("abc"), Some("-1")] {
        let attributes: Vec<(&str, &str)> = step.map(|s| ("step", s)).into_iter().collect();
        assert_eq!(shown(&attributes, 0.37), "0.37", "step {step:?}");
    }
}

#[test]
fn a_step_finer_than_six_decimals_shows_the_value_in_full() {
    assert_eq!(shown(&[("step", "0.0000001")], 0.123_456_7), "0.1234567");
    assert_eq!(shown(&[("step", "1e-9")], 0.123_456_789_1), "0.1234567891");
}

#[test]
fn typed_text_shows_as_typed() {
    let field = field(&[("step", "1")], PropertyValue::Text("7.1465".to_owned()));
    assert_eq!(field.shown, "7.1465");
}

#[test]
fn the_fields_value_keeps_the_full_number() {
    let field = field(&[("step", "1")], PropertyValue::Number(7.1465));
    assert_eq!(field.value(), "7.1465");
}

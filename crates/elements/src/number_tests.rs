//! A number field's numbers from its attributes and properties.

use std::collections::BTreeMap;

use vapors_dom::PropertyValue;

use super::{Field, typeable};

fn attributes(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

fn field(pairs: &[(&str, &str)]) -> Field {
    Field::of(&attributes(pairs), &BTreeMap::new())
}

#[test]
fn without_attributes_a_field_is_unbounded_by_1_and_empty() {
    // When
    let field = field(&[]);

    // Then
    assert_eq!((field.min, field.max, field.step), (None, None, Some(1.0)));
    assert_eq!(field.text, "");
    assert_eq!(field.value(), "");
    assert!(!field.invalid(), "an empty field is not invalid");
}

#[test]
fn the_value_is_the_text_when_it_is_a_number_else_empty() {
    // Given
    let number = field(&[("value", "-2.5e1")]);
    let text = field(&[("value", "1e")]);

    // Then
    assert_eq!(number.value(), "-2.5e1");
    assert_eq!(number.number(), Some(-25.0));
    assert_eq!(text.value(), "");
    assert!(text.invalid());
}

#[test]
fn a_number_outside_the_range_is_invalid_and_commits_clamped() {
    // Given
    let over = field(&[("min", "0"), ("max", "100"), ("value", "250")]);

    // Then
    assert!(over.invalid());
    assert_eq!(over.committed(), Some(100.0));
    assert_eq!(over.clamped(-3.0), 0.0);
    assert_eq!(field(&[("value", "1e")]).committed(), None);
}

#[test]
fn stepping_moves_by_the_step_from_the_text_or_the_fallback_clamped() {
    // Given: 0..10 by 0.5
    let near_top = field(&[
        ("min", "0"),
        ("max", "10"),
        ("step", "0.5"),
        ("value", "9.5"),
    ]);
    let blank = Field {
        text: "x".into(),
        ..near_top.clone()
    };

    // Then
    assert_eq!(near_top.stepped(1.0, 0.0), 10.0);
    assert_eq!(near_top.stepped(-10.0, 0.0), 4.5);
    assert_eq!(blank.stepped(2.0, 3.0), 4.0, "from the fallback");
    assert_eq!(super::step_of(Some("any")), None);
    assert_eq!(super::step_of(Some("-1")), Some(1.0));
}

#[test]
fn a_value_property_sets_the_text_a_number_written_and_a_text_as_is() {
    // Given
    let attributes = attributes(&[("value", "10")]);
    let number = BTreeMap::from([("value".to_owned(), PropertyValue::Number(2.5))]);
    let text = BTreeMap::from([("value".to_owned(), PropertyValue::Text("7e".into()))]);
    let nan = BTreeMap::from([("value".to_owned(), PropertyValue::Number(f64::NAN))]);

    // Then
    assert_eq!(Field::of(&attributes, &number).text, "2.5");
    assert_eq!(Field::of(&attributes, &text).text, "7e");
    assert_eq!(Field::of(&attributes, &nan).text, "", "NaN is no value");
}

#[test]
fn only_digits_signs_the_point_and_e_are_typeable() {
    assert!("0123456789+-.eE".chars().all(typeable));
    assert!(!"abc x,".chars().any(typeable));
}

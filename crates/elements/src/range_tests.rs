//! A range's numbers from its attributes and properties.

use std::collections::BTreeMap;

use vapors_dom::PropertyValue;

use super::{Range, written};

fn attributes(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

fn range(pairs: &[(&str, &str)]) -> Range {
    Range::of(&attributes(pairs), &BTreeMap::new())
}

#[test]
fn without_attributes_a_range_is_0_to_100_by_1_at_50() {
    // When
    let range = range(&[]);

    // Then
    assert_eq!((range.min, range.max, range.step), (0.0, 100.0, Some(1.0)));
    assert_eq!(range.value, 50.0);
}

#[test]
fn the_value_attribute_is_clamped_and_snapped() {
    for (pairs, expected) in [
        (vec![("value", "30")], 30.0),
        (vec![("value", "130")], 100.0),
        (vec![("value", "-3")], 0.0),
        (vec![("min", "10"), ("max", "20"), ("value", "15.4")], 15.0),
        (
            vec![
                ("min", "0"),
                ("max", "1"),
                ("step", "0.25"),
                ("value", "0.6"),
            ],
            0.5,
        ),
        // the step base is the value attribute, 9: -1, 4, 9; 9 is past
        // the maximum, so the nearest allowed is 4
        (vec![("max", "7"), ("step", "5"), ("value", "9")], 4.0),
        (vec![("value", "abc")], 50.0),
        (vec![("value", " 7")], 50.0),
        (vec![("value", "+7")], 50.0),
    ] {
        // When
        let range = range(&pairs);

        // Then
        assert_eq!(range.value, expected, "{pairs:?}");
    }
}

#[test]
fn a_max_below_min_is_min_and_the_default_value_is_min() {
    // When
    let range = range(&[("min", "40"), ("max", "10")]);

    // Then
    assert_eq!((range.min, range.max, range.value), (40.0, 40.0, 40.0));
}

#[test]
fn the_step_base_is_min_else_the_value_attribute_else_zero() {
    // Given: no min, a value of 5 and a step of 10: 5, 15, 25...
    let range = range(&[("value", "5"), ("step", "10")]);

    // Then
    assert_eq!(range.base, 5.0);
    assert_eq!(range.snapped(12.0), 15.0);
    assert_eq!(
        range.snapped(99.0),
        95.0,
        "past the maximum: the step below"
    );
}

#[test]
fn step_any_or_an_unusable_step_reads_as_none_or_the_default() {
    // Given / When: each step attribute read; Then
    assert_eq!(range(&[("step", "any")]).step, None);
    assert_eq!(range(&[("step", "ANY")]).step, None);
    assert_eq!(range(&[("step", "0")]).step, Some(1.0));
    assert_eq!(range(&[("step", "-2")]).step, Some(1.0));
    assert_eq!(range(&[("step", "x")]).step, Some(1.0));
    assert_eq!(range(&[("step", "any"), ("value", "3.3")]).value, 3.3);
}

#[test]
fn a_value_property_overrides_the_attribute_a_number_or_a_text() {
    // Given
    let attributes = attributes(&[("value", "10")]);
    let number = BTreeMap::from([("value".to_owned(), PropertyValue::Number(42.0))]);
    let text = BTreeMap::from([("value".to_owned(), PropertyValue::Text("42.6".into()))]);
    let bad = BTreeMap::from([("value".to_owned(), PropertyValue::Text(String::new()))]);

    // Then
    assert_eq!(Range::of(&attributes, &number).value, 42.0);
    assert_eq!(Range::of(&attributes, &text).value, 43.0);
    assert_eq!(
        Range::of(&attributes, &bad).value,
        50.0,
        "an empty text is no value: the default, not the attribute"
    );
}

#[test]
fn stepping_keys_move_by_the_step_and_pages_by_a_tenth_of_the_range() {
    // Given: 0..200 by 1 at 20
    let range = Range::of(
        &attributes(&[("max", "200"), ("value", "20")]),
        &BTreeMap::new(),
    );

    // Then
    assert_eq!(range.stepped(1.0), 21.0);
    assert_eq!(range.stepped(-10.0), 10.0);
    assert_eq!(range.stepped(-30.0), 0.0);
    assert_eq!(range.paged(true), 40.0, "a tenth of 200");
    assert_eq!(range.paged(false), 0.0);
    assert_eq!(
        Range::of(&attributes(&[("step", "any")]), &BTreeMap::new()).stepped(1.0),
        51.0,
        "step=any steps by a hundredth of the range"
    );
}

#[test]
fn a_fraction_of_the_track_maps_to_a_snapped_value_and_back() {
    // Given: 10..20 by 2
    let range = range(&[("min", "10"), ("max", "20"), ("step", "2"), ("value", "14")]);

    // Then
    assert_eq!(range.fraction(), 0.4);
    assert_eq!(range.at_fraction(0.33), 14.0);
    assert_eq!(range.at_fraction(1.5), 20.0);
    assert_eq!(range.at_fraction(-1.0), 10.0);
}

#[test]
fn the_value_is_written_as_the_dom_writes_it() {
    assert_eq!(written(50.0), "50");
    assert_eq!(written(12.5), "12.5");
    assert_eq!(written(-0.1), "-0.1");
}

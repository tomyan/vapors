//! The `_if_changed` setters the generated bindings call: a value the
//! node already holds is not written; any other value is, as the plain
//! setter writes it.

use vapors_dom::recording::{Call, RecordingDom};
use vapors_dom::{Dom, NodeId, PropertyValue};

/// A tree with one `<input>` and one text node, its calls cleared.
fn tree() -> (RecordingDom, NodeId, NodeId) {
    let mut dom = RecordingDom::new();
    let input = dom.create_element("input");
    let text = dom.create_text("0");
    dom.set_attribute(input, "class", "wide");
    dom.set_property(input, "value", PropertyValue::Text("Warm".into()));
    dom.clear_calls();
    (dom, input, text)
}

#[test]
fn an_equal_text_is_not_written() {
    // Given
    let (mut dom, _, text) = tree();

    // When
    dom.set_text_if_changed(text, "0");

    // Then
    assert_eq!(dom.calls(), &[]);
}

#[test]
fn a_different_text_is_written() {
    // Given
    let (mut dom, _, text) = tree();

    // When
    dom.set_text_if_changed(text, "1");

    // Then
    assert_eq!(
        dom.calls(),
        &[Call::SetText {
            node: text,
            text: "1".into()
        }]
    );
}

#[test]
fn an_equal_attribute_is_not_written() {
    // Given
    let (mut dom, input, _) = tree();

    // When
    dom.set_attribute_if_changed(input, "class", "wide");

    // Then
    assert_eq!(dom.calls(), &[]);
}

#[test]
fn a_different_or_missing_attribute_is_written() {
    // Given
    let (mut dom, input, _) = tree();

    // When
    dom.set_attribute_if_changed(input, "class", "small");
    dom.set_attribute_if_changed(input, "title", "");

    // Then
    assert_eq!(dom.calls().len(), 2, "{:?}", dom.calls());
    assert_eq!(dom.attribute(input, "title").as_deref(), Some(""));
}

#[test]
fn an_equal_property_is_not_written() {
    // Given
    let (mut dom, input, _) = tree();

    // When
    dom.set_property_if_changed(input, "value", PropertyValue::Text("Warm".into()));

    // Then
    assert_eq!(dom.calls(), &[]);
}

#[test]
fn a_property_of_another_kind_or_value_is_written() {
    // Given
    let (mut dom, input, _) = tree();

    // When
    dom.set_property_if_changed(input, "value", PropertyValue::Number(1.0));
    dom.set_property_if_changed(input, "checked", PropertyValue::Bool(false));

    // Then
    assert_eq!(dom.calls().len(), 2, "{:?}", dom.calls());
}

#[test]
fn a_nan_property_is_written_every_time() {
    // Given
    let (mut dom, input, _) = tree();
    dom.set_property(input, "value", PropertyValue::Number(f64::NAN));
    dom.clear_calls();

    // When
    dom.set_property_if_changed(input, "value", PropertyValue::Number(f64::NAN));

    // Then
    assert_eq!(dom.calls().len(), 1, "{:?}", dom.calls());
}

#[test]
fn zero_to_negative_zero_is_written() {
    // Given: a range shows "0" for 0.0 and "-0" for -0.0
    let (mut dom, input, _) = tree();
    dom.set_property(input, "value", PropertyValue::Number(0.0));
    dom.clear_calls();

    // When
    dom.set_property_if_changed(input, "value", PropertyValue::Number(-0.0));

    // Then
    assert_eq!(dom.calls().len(), 1, "{:?}", dom.calls());
}

#[test]
fn negative_zero_to_zero_is_written() {
    // Given
    let (mut dom, input, _) = tree();
    dom.set_property(input, "value", PropertyValue::Number(-0.0));
    dom.clear_calls();

    // When
    dom.set_property_if_changed(input, "value", PropertyValue::Number(0.0));

    // Then
    assert_eq!(dom.calls().len(), 1, "{:?}", dom.calls());
}

#[test]
fn an_equal_number_is_not_written() {
    // Given
    let (mut dom, input, _) = tree();
    dom.set_property(input, "value", PropertyValue::Number(2.5));
    dom.clear_calls();

    // When
    dom.set_property_if_changed(input, "value", PropertyValue::Number(2.5));

    // Then
    assert_eq!(dom.calls(), &[]);
}

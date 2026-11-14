//! A refused call in `RecordingDom` panics where it is made, naming
//! it, unless the test said it expects refusals; then it is recorded
//! and read through `refusals()`.

use vapors_dom::recording::{Call, RecordingDom};
use vapors_dom::{Dom, StyleKey};

static ROW: StyleKey = StyleKey {
    site: "row.wisp:1:1",
    scope: None,
    tag: "div",
    classes: &["row"],
    toggles: &[],
    attributes: &[],
    style: None,
};

#[test]
#[should_panic(
    expected = "RecordingDom refused a call: set_attribute: `class` on <div> at row.wisp:1:1 is set from its template"
)]
fn a_refused_call_panics_where_it_is_made_when_none_is_expected() {
    let mut dom = RecordingDom::new();
    let div = dom.create_keyed_element(&ROW);
    dom.set_attribute(div, "class", "other");
}

#[test]
fn an_expected_refusal_is_recorded_and_read_once() {
    // Given
    let mut dom = RecordingDom::new();
    dom.expect_refusals();
    let div = dom.create_keyed_element(&ROW);

    // When
    dom.set_attribute(div, "class", "other");

    // Then
    assert_eq!(dom.refusals().len(), 1);
    assert_eq!(dom.refusals(), Vec::<String>::new());
    assert!(matches!(dom.calls().last(), Some(Call::Refused { node, .. }) if *node == div));
}

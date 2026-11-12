//! A refusal `RecordingDom` records must be read: dropping the
//! recording with one no test took through `refusals()` panics, naming
//! it, unless the test is already failing.

use vapors_dom::recording::RecordingDom;
use vapors_dom::{Dom, StyleKey};

static ROW: StyleKey = StyleKey {
    site: "row.wisp:1:1",
    scope: None,
    tag: "div",
    classes: &["row"],
    toggles: &[],
    style: None,
};

/// A recording holding one refused `class` write.
fn refused() -> RecordingDom {
    let mut dom = RecordingDom::new();
    let div = dom.create_keyed_element(&ROW);
    dom.set_attribute(div, "class", "other");
    dom
}

#[test]
#[should_panic(
    expected = "RecordingDom dropped with 1 refused call(s) no test read: set_attribute: `class` on <div> at row.wisp:1:1 is set from its template"
)]
fn dropping_a_recording_with_an_unread_refusal_panics_naming_it() {
    drop(refused());
}

#[test]
fn a_refusal_taken_through_refusals_is_read() {
    // Given
    let mut dom = refused();

    // When
    let taken = dom.refusals();

    // Then: dropping it passes
    assert_eq!(taken.len(), 1);
    drop(dom);
}

#[test]
#[should_panic(expected = "the test's own failure")]
fn a_failing_test_reports_its_own_failure_not_the_unread_refusal() {
    let _dom = refused();
    panic!("the test's own failure");
}

//! Setting a value through the a11y tree (R1) as Chromium's
//! accessibility set-value does (`a11y_set_value.rs` cites the source):
//! a range's value clamped and snapped, a number's kept as given, and
//! one `input` and one `change` when the value moved, none when not.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::drive::{Log, log, node, shared, target, update};
use support::roles_page::page;
use vapors_dom::Dom;
use vapors_elements::{AxError, Document};

/// Sets `value` on `role` named `name`; the events it fired and the
/// value after.
fn set(doc: &Rc<RefCell<Document>>, role: &str, name: &str, value: &str) -> (Log, String) {
    let node = node(doc, role, name);
    let events = log(doc, node, &["input", "change"]);
    Document::ax_set_value(doc, &target(role, name), value).expect("set");
    let after = doc.borrow().value(node).expect("a control");
    (events, after)
}

#[test]
fn a_range_takes_the_value_snapped_with_one_input_and_one_change() {
    // Given
    let doc = shared(page());

    // When: between steps of 1
    let (events, value) = set(&doc, "slider", "Amount", "73.6");

    // Then
    assert_eq!(value, "74");
    assert_eq!(*events.borrow(), ["input", "change"]);
}

#[test]
fn a_range_clamps_a_value_past_its_maximum() {
    // Given
    let doc = shared(page());

    // When
    let (_, value) = set(&doc, "slider", "Amount", "150");

    // Then
    assert_eq!(value, "100");
}

#[test]
fn a_value_that_moves_nothing_fires_nothing() {
    // Given: Amount is 50 and Radius 2
    let doc = shared(page());

    // When
    let (range_events, _) = set(&doc, "slider", "Amount", "50.2");
    update(&doc);
    let (number_events, _) = set(&doc, "spinbutton", "Radius", "2");

    // Then
    assert!(range_events.borrow().is_empty());
    assert!(number_events.borrow().is_empty());
}

#[test]
fn a_number_keeps_the_text_as_given_neither_clamped_nor_stepped() {
    // Given: Radius at most 5
    let doc = shared(page());
    let radius = node(&doc, "spinbutton", "Radius");
    doc.borrow_mut().set_attribute(radius, "max", "5");
    update(&doc);

    // When
    let (events, value) = set(&doc, "spinbutton", "Radius", "7.50");

    // Then
    assert_eq!(value, "7.50");
    assert_eq!(*events.borrow(), ["input", "change"]);
}

#[test]
fn a_number_written_differently_is_a_change_as_html_compares_strings() {
    // Given: Radius set to 3
    let doc = shared(page());
    set(&doc, "spinbutton", "Radius", "3");
    update(&doc);

    // When: the same number, written another way
    let (events, value) = set(&doc, "spinbutton", "Radius", "3.0");

    // Then
    assert_eq!(value, "3.0");
    assert_eq!(*events.borrow(), ["input", "change"]);
}

#[test]
fn text_that_is_no_number_is_refused_naming_it() {
    // Given
    let doc = shared(page());

    // When
    let refused = Document::ax_set_value(&doc, &target("slider", "Amount"), "half");

    // Then
    let not_a_number = AxError::NotANumber {
        role: "slider".to_owned(),
        name: "Amount".to_owned(),
        value: "half".to_owned(),
    };
    assert_eq!(refused, Err(not_a_number));
}

#[test]
fn a_value_on_a_node_with_none_to_set_is_refused() {
    // Given
    let doc = shared(page());

    // When
    let refused = Document::ax_set_value(&doc, &target("textbox", "Note"), "1");

    // Then
    let not_settable = AxError::NotSettable {
        role: "textbox".to_owned(),
        name: "Note".to_owned(),
    };
    assert_eq!(refused, Err(not_settable));
}

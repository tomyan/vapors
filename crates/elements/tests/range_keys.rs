//! A range input by key and by property (3-E3a): arrow keys step by
//! one step and Shift by ten, Page, Home and End as Chromium, each
//! firing `input` and `change`; a `value` property sets it silently.

mod support;

use support::range_page::{key, page, page_with, shift_key, value};
use vapors_dom::{Dom, Event, PropertyValue};

#[test]
fn arrow_keys_step_by_one_and_shift_by_ten_firing_input_and_change() {
    // Given: focused by Tab
    let page = page();
    key(&page, "Tab");

    // When
    key(&page, "ArrowRight");
    shift_key(&page, "ArrowRight");
    shift_key(&page, "ArrowLeft");
    key(&page, "ArrowDown");

    // Then
    assert_eq!(value(&page, page.level), "20");
    assert_eq!(
        *page.log.borrow(),
        [
            "input 21 21",
            "change 21 21",
            "input 31 31",
            "change 31 31",
            "input 21 21",
            "change 21 21",
            "input 20 20",
            "change 20 20"
        ]
    );
}

#[test]
fn a_key_at_the_end_of_the_range_changes_nothing_and_fires_nothing() {
    // Given: at 100
    let page = page_with(&[("value", "100")]);
    key(&page, "Tab");

    // When
    key(&page, "ArrowRight");
    key(&page, "End");

    // Then
    assert_eq!(value(&page, page.level), "100");
    assert!(page.log.borrow().is_empty());
}

#[test]
fn a_range_disabled_while_focused_takes_no_key() {
    // Given: focused, then disabled and updated
    let page = page();
    key(&page, "Tab");
    page.doc
        .borrow_mut()
        .set_attribute(page.level, "disabled", "");
    page.doc
        .borrow_mut()
        .update(vapors_elements::Viewport {
            width: 400.0,
            height: 100.0,
            scale: 1.0,
        })
        .expect("updates");

    // When
    key(&page, "ArrowRight");

    // Then
    assert_eq!(value(&page, page.level), "20");
    assert!(page.log.borrow().is_empty());
}

#[test]
fn a_prevented_keydown_does_not_step() {
    // Given
    let page = page();
    key(&page, "Tab");
    page.doc.borrow_mut().listen(
        page.level,
        "keydown",
        Box::new(|e: &mut Event| e.prevent_default()),
    );

    // When
    key(&page, "ArrowRight");

    // Then
    assert_eq!(value(&page, page.level), "20");
}

#[test]
fn a_value_property_sets_the_value_snapped_and_a_text_too() {
    // Given
    let page = page();

    // When
    page.doc
        .borrow_mut()
        .set_property(page.level, "value", PropertyValue::Number(42.4));
    let number = value(&page, page.level);
    page.doc
        .borrow_mut()
        .set_property(page.level, "value", PropertyValue::Text("250".into()));

    // Then: a listener's set fires no event
    assert_eq!(number, "42");
    assert_eq!(value(&page, page.level), "100");
    assert_eq!(page.doc.borrow().value_as_number(page.level), Some(100.0));
    assert!(page.log.borrow().is_empty());
}

#[test]
fn a_value_is_none_for_an_element_that_is_not_a_control() {
    // Given
    let page = page();
    let body = page.doc.borrow().body();

    // Then
    assert_eq!(page.doc.borrow().value(body), None);
    assert_eq!(Event::new("click").value_as_number(), None);
}

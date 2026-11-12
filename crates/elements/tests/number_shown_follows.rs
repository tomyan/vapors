//! A number field's editor holds what the field shows, and what it shows
//! depends on `step`, `min` and `max` as well as `value`
//! (`number_shown.rs`): changing one of them while the field is focused
//! or not leaves editing on the new shown text (review of vapors#133).

mod support;

use support::number_page::{Page, click, key, page_with, type_text, value};
use support::viewport;
use vapors_dom::{Dom, PropertyValue};

/// `amount` by 0.01 up to 1, set to the number 0.5000001, so it shows
/// "0.50".
fn showing_rounded() -> Page {
    let page = page_with(&[
        ("min", "0"),
        ("max", "1"),
        ("step", "0.01"),
        ("value", "0.5"),
    ]);
    set_number(&page, 0.500_000_1);
    page
}

fn set_number(page: &Page, number: f64) {
    page.doc
        .borrow_mut()
        .set_property(page.amount, "value", PropertyValue::Number(number));
    settle(page);
}

fn set_attribute(page: &Page, name: &str, text: &str) {
    page.doc.borrow_mut().set_attribute(page.amount, name, text);
    settle(page);
}

fn settle(page: &Page) {
    page.doc
        .borrow_mut()
        .update(viewport(400.0, 100.0))
        .expect("updates");
}

#[test]
fn a_max_set_below_the_value_after_a_blur_edits_the_full_text() {
    // Given: focused by a click and blurred, then the number set
    let page = page_with(&[
        ("min", "0"),
        ("max", "1"),
        ("step", "0.01"),
        ("value", "0.5"),
    ]);
    click(&page, [50.0, 12.0]);
    click(&page, [50.0, 60.0]);
    set_number(&page, 0.500_000_1);

    // When: out of range, the field shows the full number
    set_attribute(&page, "max", "0.4");
    click(&page, [95.0, 12.0]);
    key(&page, "End");
    type_text(&page, "9");

    // Then
    assert_eq!(value(&page, page.amount), "0.50000019");
}

#[test]
fn a_step_set_to_any_while_focused_edits_the_full_text() {
    // Given: focused, the caret at the end of "0.50"
    let page = showing_rounded();
    click(&page, [95.0, 12.0]);

    // When: with no usable step, the field shows the full number
    set_attribute(&page, "step", "any");
    key(&page, "End");
    type_text(&page, "9");

    // Then
    assert_eq!(value(&page, page.amount), "0.50000019");
}

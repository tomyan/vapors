//! A number field shows a value set as a number at its step's precision
//! (`number_shown.rs`), on the painted page; its DOM value keeps the
//! full number, editing starts from what is shown, and a step lands on
//! the steps. Chromium paints the full number; this differs on
//! purpose, for display only (touché, ux-review-1).

mod support;

use support::number_page::{Page, click, key, page, type_text, value};
use support::viewport;
use vapors_dom::{Dom, PropertyValue};
use vapors_scene::Item;

/// `amount` (0..100 by 1) set to the number 7.1465, as an app's signal
/// sets it.
fn set_off_the_steps() -> Page {
    let page = page();
    page.doc
        .borrow_mut()
        .set_property(page.amount, "value", PropertyValue::Number(7.1465));
    page.doc
        .borrow_mut()
        .update(viewport(400.0, 100.0))
        .expect("updates");
    page
}

fn painted_texts(page: &Page) -> Vec<String> {
    let doc = page.doc.borrow();
    let painted = doc.painted().expect("painted");
    painted
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, .. } => Some(painted.text(*text).text().to_owned()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_number_set_on_a_field_by_1_paints_whole_and_keeps_its_value() {
    // When
    let page = set_off_the_steps();

    // Then
    let texts = painted_texts(&page);
    assert!(texts.iter().any(|t| t == "7"), "{texts:?}");
    assert!(!texts.iter().any(|t| t.contains("7.1465")), "{texts:?}");
    assert_eq!(value(&page, page.amount), "7.1465");
}

#[test]
fn backspace_after_a_click_deletes_the_shown_digit_not_a_hidden_one() {
    // Given: focused by a click, the caret at the end of "7"
    let page = set_off_the_steps();
    click(&page, [50.0, 12.0]);

    // When
    key(&page, "Backspace");

    // Then
    assert_eq!(value(&page, page.amount), "");
}

#[test]
fn typing_after_a_click_appends_to_the_shown_text() {
    // Given
    let page = set_off_the_steps();
    click(&page, [50.0, 12.0]);

    // When
    type_text(&page, "5");

    // Then
    assert_eq!(value(&page, page.amount), "75");
}

#[test]
fn arrows_from_a_value_between_the_steps_land_on_the_steps() {
    // Given
    let page = set_off_the_steps();
    click(&page, [50.0, 12.0]);

    // When / Then: as Chromium's stepUp, the next step up, then down
    key(&page, "ArrowUp");
    assert_eq!(value(&page, page.amount), "8");
    key(&page, "ArrowDown");
    assert_eq!(value(&page, page.amount), "7");
}

#[test]
fn a_click_and_blur_without_an_edit_keeps_the_full_number() {
    // Given
    let page = set_off_the_steps();
    click(&page, [50.0, 12.0]);

    // When: a click on the disabled field blurs amount
    click(&page, [50.0, 60.0]);

    // Then
    assert_eq!(value(&page, page.amount), "7.1465");
    assert!(page.log.borrow().is_empty(), "{:?}", page.log.borrow());
}

//! Box marks (IU3 PR 1b): each of the person's edits marks the field it
//! writes, and a caret or selection change in the focused field marks
//! that field, so the update after it matches a full update. Each case
//! fails without its mark. Each page is updated at the viewport it was
//! built at, since a viewport change marks everything.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::oracle::assert_matches_full;
use support::{checkbox_page, number_page, range_page, text_page, viewport};
use vapors_elements::{Document, Viewport};

/// The viewport the number, range and text pages update at.
fn field_view() -> Viewport {
    viewport(400.0, 100.0)
}

fn update(doc: &Rc<RefCell<Document>>, at: Viewport) {
    doc.borrow_mut().update(at).expect("updates");
}

#[test]
fn a_checkbox_click_matches_a_full_update() {
    // Given
    let page = checkbox_page::page();

    // When
    checkbox_page::click(&page, [5.0, 5.0]);

    // Then
    assert_matches_full(&page.doc, viewport(100.0, 100.0), "checkbox click");
}

/// Pressing Delete with the caret at the start changes the text and
/// leaves the caret's rect where it was, so the field's text marks do
/// not change and only the edit's own mark rebuilds its box.
#[test]
fn a_digit_deleted_in_a_number_field_matches_a_full_update() {
    // Given
    let page = number_page::page();
    number_page::click(&page, [50.0, 12.0]);
    number_page::key(&page, "Home");
    update(&page.doc, field_view());

    // When
    number_page::key(&page, "Delete");

    // Then
    assert_eq!(number_page::value(&page, page.amount), "0");
    assert_matches_full(&page.doc, field_view(), "number delete");
}

#[test]
fn a_range_stepped_by_a_key_matches_a_full_update() {
    // Given
    let page = range_page::page();
    Document::focus(&page.doc, Some(page.level));
    // the first key press makes the focus visible, which restyles
    range_page::key(&page, "ArrowRight");
    update(&page.doc, field_view());

    // When
    range_page::key(&page, "ArrowRight");

    // Then
    assert_matches_full(&page.doc, field_view(), "range key");
}

/// The text page with `name` focused by a click and updated, so the
/// focused field's text marks are kept.
fn focused_text_page() -> text_page::Page {
    let page = text_page::page();
    text_page::click(&page, [80.0, 12.0]);
    update(&page.doc, field_view());
    page
}

/// As with the number field, Delete at the start leaves the caret's
/// rect where it was.
#[test]
fn a_character_deleted_in_a_text_field_matches_a_full_update() {
    // Given
    let page = focused_text_page();
    text_page::key(&page, "Home");
    update(&page.doc, field_view());

    // When
    text_page::key(&page, "Delete");

    // Then
    assert_eq!(text_page::value(&page, page.name), "arm");
    assert_matches_full(&page.doc, field_view(), "text delete");
}

#[test]
fn a_caret_move_in_a_focused_field_matches_a_full_update() {
    // Given
    let page = focused_text_page();

    // When
    text_page::key(&page, "ArrowLeft");

    // Then
    assert_matches_full(&page.doc, field_view(), "caret move");
}

#[test]
fn a_selection_change_in_a_focused_field_matches_a_full_update() {
    // Given
    let page = focused_text_page();
    let (key, modifiers) = text_page::shift("ArrowLeft");

    // When
    text_page::key_with(&page, key, modifiers);

    // Then
    assert_matches_full(&page.doc, field_view(), "selection change");
}

//! The focused text field as the platform's text input sees it (the
//! web page's hidden textarea): its text, selection in UTF-16 units and
//! caret area for the platform to mirror; and the textarea's edits,
//! composition included, coming back as the field's new text, with
//! `input`, the composing range underlined. An edit made against a text
//! the field no longer holds is refused, naming both.

mod support;

use support::text_page::{click, page, selection, value};
use vapors_elements::{Document, TextEdit};

fn edit(
    before: &str,
    text: &str,
    selection: (usize, usize),
    composing: Option<(usize, usize)>,
) -> TextEdit {
    TextEdit {
        before: before.to_owned(),
        text: text.to_owned(),
        selection,
        composing,
    }
}

#[test]
fn an_unfocused_document_has_no_text_input() {
    // Given
    let page = page();

    // Then
    assert_eq!(page.doc.borrow_mut().text_input(), None);
}

#[test]
fn a_focused_field_offers_its_text_selection_and_caret_area() {
    // Given: the caret after "Warm"
    let page = page();
    click(&page, [120.0, 12.0]);

    // When
    let input = page.doc.borrow_mut().text_input().expect("focused");

    // Then: the area is the caret's, inside the field
    assert_eq!(input.text, "Warm");
    assert_eq!(input.selection, (4, 4));
    let [x, y, width, height] = input.area;
    assert!(
        x > 30.0 && x < 50.0 && y >= 0.0 && width >= 1.0 && height > 10.0 && y + height <= 24.0,
        "{:?}",
        input.area
    );
}

#[test]
fn a_composing_edit_is_in_the_value_with_input() {
    // Given
    let page = page();
    click(&page, [120.0, 12.0]);
    page.log.borrow_mut().clear();

    // When: an IME composes "th" after "Warm"
    let applied = Document::text_edited(&page.doc, &edit("Warm", "Warmth", (6, 6), Some((4, 6))));

    // Then
    assert_eq!(applied, Ok(()));
    assert_eq!(value(&page, page.name), "Warmth");
    assert_eq!(selection(&page, page.name), (6, 6));
    assert_eq!(*page.log.borrow(), ["input \"Warmth\""]);
}

#[test]
fn a_deletion_from_the_textarea_reaches_the_field() {
    // Given
    let page = page();
    click(&page, [120.0, 12.0]);

    // When: Backspace as Gboard sends it, through the textarea
    let applied = Document::text_edited(&page.doc, &edit("Warm", "War", (3, 3), None));

    // Then
    assert_eq!(applied, Ok(()));
    assert_eq!(value(&page, page.name), "War");
}

#[test]
fn an_edit_against_another_text_is_refused_naming_both() {
    // Given: the field holds "Warm"
    let page = page();
    click(&page, [120.0, 12.0]);

    // When
    let refused = Document::text_edited(&page.doc, &edit("Cold", "Colder", (6, 6), None));

    // Then
    let why = refused.expect_err("refused");
    assert!(
        why.contains("\"Cold\"") && why.contains("\"Warm\""),
        "{why}"
    );
    assert_eq!(value(&page, page.name), "Warm");
}

#[test]
fn an_edit_with_no_field_focused_is_refused() {
    // Given
    let page = page();

    // When
    let refused = Document::text_edited(&page.doc, &edit("", "a", (1, 1), None));

    // Then
    assert!(refused.expect_err("refused").contains("no text field"));
}

#[test]
fn a_selection_past_the_text_is_refused() {
    // Given
    let page = page();
    click(&page, [120.0, 12.0]);

    // When
    let refused = Document::text_edited(&page.doc, &edit("Warm", "Warmth", (9, 9), None));

    // Then
    assert!(refused.expect_err("refused").contains("selection"));
    assert_eq!(value(&page, page.name), "Warm");
}

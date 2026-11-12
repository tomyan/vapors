//! Keys and text after the focused field was hidden since the last
//! settle (review-fable-input M5): the focus fixup runs first, as
//! Chromium's style update before the task would have blurred the
//! field, so the field commits and blurs and the key or text reaches
//! no field. An agent's edits arrive between frames, so the next frame
//! is too late.

mod support;

use support::fixup::{hide_field, typed_in};
use support::text_page::{Page, key, value};
use vapors_elements::{Document, TextEdit};

/// Chromium's events for the field "erWarm" losing focus.
const BLURRED: [&str; 3] = ["change \"erWarm\"", "blur", "focusout"];

/// The page typed in, its field hidden by an attribute since the settle.
fn hidden_since_settle() -> Page {
    let page = typed_in();
    hide_field(&page);
    page
}

#[test]
fn a_key_after_the_field_is_hidden_blurs_it_first_and_types_nothing() {
    // Given
    let page = hidden_since_settle();

    // When
    key(&page, "x");
    Document::character_typed(&page.doc, "x");

    // Then
    assert_eq!(*page.log.borrow(), BLURRED);
    assert_eq!(page.doc.borrow().active_element(), None);
    assert_eq!(value(&page, page.name), "erWarm");
}

#[test]
fn a_paste_after_the_field_is_hidden_blurs_it_first_and_is_the_apps() {
    // Given
    let page = hidden_since_settle();

    // When
    let taken = Document::text_pasted(&page.doc, "x");

    // Then
    assert!(!taken);
    assert_eq!(*page.log.borrow(), BLURRED);
    assert_eq!(value(&page, page.name), "erWarm");
}

#[test]
fn a_text_edit_after_the_field_is_hidden_blurs_it_first_and_reaches_no_field() {
    // Given
    let page = hidden_since_settle();
    let edit = TextEdit {
        before: "erWarm".into(),
        text: "erWarmx".into(),
        selection: (7, 7),
        composing: None,
    };

    // When
    let edited = Document::text_edited(&page.doc, &edit);

    // Then: an ordinary user action, not an error to report
    assert_eq!(edited, Ok(()));
    assert_eq!(*page.log.borrow(), BLURRED);
    assert_eq!(value(&page, page.name), "erWarm");
}

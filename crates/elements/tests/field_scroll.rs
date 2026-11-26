//! A text field's text scrolls inside its content box (entry 44), as
//! Chromium 157's does on the long-input page (`support::long_field`):
//! typing past the right edge scrolls just far enough that the caret
//! stays in the content box's last column, deleting brings the text
//! back as the range shrinks, and the text is cut off at the content
//! box. Blur scrolls the field back to its start; focus again shows
//! the caret. The glyph widths are vapors' (Geist, not Chromium's
//! sans-serif), so the text width differs a little; the caret's column
//! and whether the field scrolls at all are compared.

use crate::support;

use support::long_field::{
    CHROMIUM_DELETED, CHROMIUM_TYPED, CONTENT, Page, backspace, caret_column, length, offset, page,
    text_clip, type_w, update,
};
use vapors_elements::{Document, Modifiers};

/// Types up to `count` characters.
fn typed_to(page: &Page, count: usize) {
    while length(page) < count {
        type_w(page);
    }
}

/// Asserts the field matches Chromium's `(length, scrollLeft, caret
/// column)` at its current length: scrolled exactly when Chromium's is,
/// and, once scrolled, the caret within a column of Chromium's.
fn assert_like_chromium(page: &Page, (count, left, column): (usize, f32, f32), op: &str) {
    let (at, shown) = (offset(page), caret_column(page));
    assert_eq!(
        at > 0.0,
        left > 0.0,
        "{op} {count}: offset {at}, Chromium {left}"
    );
    let shown = shown.unwrap_or_else(|| panic!("{op} {count}: no caret shows (offset {at})"));
    assert!(
        shown >= CONTENT.0 && shown < CONTENT.1,
        "{op} {count}: caret at {shown}, outside {CONTENT:?}"
    );
    if left > 0.0 {
        assert!(
            (shown - column).abs() <= 1.0,
            "{op} {count}: caret at {shown}, Chromium {column}"
        );
    }
}

#[test]
fn typing_past_the_right_edge_keeps_the_caret_in_the_last_column() {
    // Given
    let page = page();

    for expected in CHROMIUM_TYPED {
        // When
        typed_to(&page, expected.0);

        // Then
        assert_like_chromium(&page, expected, "typed");
    }
}

#[test]
fn deleting_brings_the_text_back_as_the_range_shrinks() {
    // Given: 60 characters typed
    let page = page();
    typed_to(&page, 60);

    for expected in CHROMIUM_DELETED {
        // When
        while length(&page) > expected.0 {
            backspace(&page);
        }

        // Then
        assert_like_chromium(&page, expected, "deleted to");
    }
}

#[test]
fn the_text_is_cut_off_at_the_content_box() {
    // Given
    let page = page();

    // When
    typed_to(&page, 60);

    // Then: Chromium's text ink spans 689..942, its content box
    let clip = text_clip(&page).expect("the text is clipped");
    assert_eq!((clip.left, clip.right), CONTENT);
}

#[test]
fn blur_scrolls_back_to_the_start_and_focus_shows_the_caret_again() {
    // Given: 60 characters typed, scrolled to the caret
    let page = page();
    typed_to(&page, 60);

    // When
    Document::focus(&page.doc, None);
    update(&page);
    let blurred = (offset(&page), caret_column(&page));
    Document::focus(&page.doc, Some(page.field));
    update(&page);

    // Then: Chromium 0 after blur(), 539 and the caret at 941 after focus()
    assert_eq!(blurred, (0.0, None));
    assert_like_chromium(&page, (60, 539.0, 941.0), "focused again at");
}

#[test]
fn home_scrolls_back_to_show_the_caret_at_the_start() {
    // Given: 60 characters typed, scrolled to the caret at the end
    let page = page();
    typed_to(&page, 60);

    // When
    Document::key_pressed(&page.doc, "Home", Modifiers::default());
    Document::key_released(&page.doc, "Home", Modifiers::default());
    update(&page);

    // Then: unscrolled, the caret at the content box's left
    assert_eq!((offset(&page), caret_column(&page)), (0.0, Some(CONTENT.0)));
}

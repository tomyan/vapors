//! A text field's author `overflow: hidden`, `overflow-x: hidden` or
//! `overflow: clip` leaves its text scrolling to the caret, as in
//! Chromium 157 (`node scripts/chromium-field-scroll.mjs overflow`):
//! under each, typing 60 characters scrolls the field to 538 with the
//! caret in column 942 and the ink cut at the content box, and blur
//! scrolls it back to 0, exactly as with no author overflow.
//!
//! `overflow: visible` is kept as before: Chromium scrolls the text
//! under it too, but a rounded field with visible overflow and side
//! padding short of its corners loaded before (no clip), and would be
//! refused as a rounded clip if its text scrolled.

use crate::support;

use support::long_field::{
    CONTENT, Page, SHEET, caret_column, length, offset, page_with, text_clip, type_w, update,
};
use vapors_elements::Document;
use vapors_scene::{Color, Item};

const CLIPPING: [&str; 3] = ["overflow: hidden", "overflow-x: hidden", "overflow: clip"];

/// The long-input page with the author's `declarations` on the field,
/// 60 characters typed.
fn typed(declarations: &str) -> Page {
    let sheet = format!("{SHEET} input {{ {declarations} }}");
    let page = page_with(&sheet, 1280.0);
    for _ in 0..60 {
        type_w(&page);
    }
    assert_eq!(length(&page), 60, "{declarations}: typed");
    page
}

#[test]
fn a_clipping_overflow_still_scrolls_the_text_to_the_caret() {
    for overflow in CLIPPING {
        // Given / When
        let page = typed(overflow);

        // Then: Chromium 538, the caret at 942, ink cut at 689..942
        assert!(offset(&page) > 0.0, "{overflow}: offset 0");
        let column = caret_column(&page).unwrap_or_else(|| panic!("{overflow}: no caret shows"));
        assert!(
            (column - 942.0).abs() <= 1.0,
            "{overflow}: caret at {column}"
        );
        let clip = text_clip(&page).unwrap_or_else(|| panic!("{overflow}: text unclipped"));
        assert_eq!((clip.left, clip.right), CONTENT, "{overflow}");
    }
}

#[test]
fn a_clipping_overflow_still_scrolls_back_on_blur() {
    for overflow in CLIPPING {
        // Given
        let page = typed(overflow);

        // When
        Document::focus(&page.doc, None);
        update(&page);

        // Then: Chromium 0
        assert_eq!(offset(&page), 0.0, "{overflow}");
    }
}

#[test]
fn a_visible_overflow_leaves_the_text_unscrolled_as_before() {
    // Given / When
    let page = typed("overflow: visible;");

    // Then: no scroll offset at all
    assert_eq!(page.doc.borrow().scroll_offset(page.field), None);
}

#[test]
fn a_field_shorter_than_its_line_with_a_y_scroll_draws_no_thumb() {
    // Given: Chromium draws no thumb on an input whatever its overflow;
    // square, so a y-scrolling field loaded before this change too
    let sheet = format!(
        "{SHEET} input {{ border-radius: 0; overflow-x: hidden; overflow-y: auto; height: 12px; scrollbar-color: #00ff00 transparent; }}"
    );
    let page = page_with(&sheet, 1280.0);

    // When
    let doc = page.doc.borrow();
    let green = Color {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    let thumbs = doc.painted().expect("updated").scene().items();
    let count = thumbs
        .filter(|(item, _)| matches!(item, Item::Quad(q) if q.background == green))
        .count();

    // Then
    assert_eq!(count, 0, "a thumb is drawn");
}

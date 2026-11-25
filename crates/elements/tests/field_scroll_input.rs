//! A scrolled text field's text under the pointer, the wheel and the
//! platform's text input (entry 44), on the long-input page
//! (`support::long_field`): a click places the caret under the
//! pointer, scrolled or not; a horizontal wheel scrolls the field, focused
//! or not, while a vertical one passes it by, as Chromium 157's does;
//! the IME's caret area is the painted caret's; and a narrower window
//! keeps the caret in the content box on its first frame, with no
//! edit.

use crate::support;

use support::long_field::{
    Page, SHEET, border_box, caret_column, click, length, offset, page, page_with, type_w, update,
};
use vapors_elements::Document;

/// Half a "W" in 14 px Geist (13.09 px): how far a click's caret can
/// be from the pointer.
const HALF_GLYPH: f32 = 6.6;

fn typed_to(page: &Page, count: usize) {
    while length(page) < count {
        type_w(page);
    }
}

fn middle_y(page: &Page) -> f32 {
    let [_, y, _, h] = border_box(page);
    y + h / 2.0
}

#[test]
fn a_click_in_a_scrolled_field_places_the_caret_under_the_pointer() {
    // Given: 60 characters, scrolled to the end
    let page = page();
    typed_to(&page, 60);
    let scrolled = offset(&page);

    // When
    click(&page, [800.0, middle_y(&page)]);

    // Then: the caret shows within half a glyph of the click, unscrolled
    let column = caret_column(&page).expect("a caret shows");
    assert!((column - 800.0).abs() <= HALF_GLYPH, "caret at {column}");
    assert_eq!(offset(&page), scrolled);
}

#[test]
fn a_click_after_blur_places_the_caret_from_the_start() {
    // Given: 60 characters, then blurred, which scrolls back
    let page = page();
    typed_to(&page, 60);
    Document::focus(&page.doc, None);
    update(&page);

    // When
    click(&page, [800.0, middle_y(&page)]);

    // Then: Chromium's scrollLeft 0 and its caret's x 794.7
    let column = caret_column(&page).expect("a caret shows");
    assert!((column - 800.0).abs() <= HALF_GLYPH, "caret at {column}");
    assert_eq!(offset(&page), 0.0);
}

#[test]
fn a_horizontal_wheel_scrolls_a_blurred_field_and_a_vertical_one_passes_it_by() {
    // Given: 60 characters, blurred
    let page = page();
    typed_to(&page, 60);
    Document::focus(&page.doc, None);
    update(&page);
    let point = [800.0, middle_y(&page)];

    // When
    let vertical = page.doc.borrow_mut().scroll_by(point, [0.0, 100.0]);
    let horizontal = page.doc.borrow_mut().scroll_by(point, [100.0, 0.0]);
    update(&page);

    // Then: Chromium's scrollLeft 100 after deltaX 100, unmoved by deltaY
    assert_eq!((vertical, horizontal), (false, true));
    assert_eq!(offset(&page), 100.0);
}

#[test]
fn the_text_inputs_caret_area_is_the_painted_caret() {
    // Given: 60 characters, scrolled to the end
    let page = page();
    typed_to(&page, 60);

    // When
    let input = page.doc.borrow_mut().text_input().expect("focused");

    // Then
    let column = caret_column(&page).expect("a caret shows");
    assert_eq!(input.area[0].round(), column);
}

#[test]
fn a_narrower_window_keeps_the_caret_in_the_content_box_on_its_first_frame() {
    // Given: a field half the window wide, 30 characters fitting at 1280
    let sheet = SHEET
        .replace("left: 676px", "left: 0")
        .replace("width: 280px", "width: 50%");
    let mut page = page_with(&sheet, 1280.0);
    typed_to(&page, 30);
    assert_eq!(offset(&page), 0.0, "fits at 1280");

    // When: the window narrows to 400, a 200 px field, with no edit
    page.width = 400.0;
    update(&page);

    // Then: its content box is 13..187 and the caret shows in it
    let column = caret_column(&page).expect("a caret shows");
    assert!((13.0..187.0).contains(&column), "caret at {column}");
}

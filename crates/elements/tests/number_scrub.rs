//! A number field's scrub (3-E3c): with `data-scrub`, a drag along the
//! field steps the number per 2 px (or the px the attribute names),
//! with `input` per step and `change` at the release; a press without
//! movement is a click that focuses; a finger across the field is the
//! pan's; without the attribute a drag does nothing.

mod support;

use support::number_page::{page, value};
use vapors_dom::Dom;
use vapors_elements::Document;

#[test]
fn a_drag_along_a_scrubbable_field_steps_per_two_px_and_changes_at_the_end() {
    // Given: `scrub` at y 24..48, at 50
    let page = page();

    // When: pressed, dragged 10 px right, 4 px back, released
    Document::pointer_pressed(&page.doc, [50.0, 36.0]);
    Document::pointer_moved(&page.doc, [60.0, 36.0]);
    Document::pointer_moved(&page.doc, [56.0, 36.0]);
    Document::pointer_released(&page.doc, [56.0, 36.0]);

    // Then: 55, 53, committed 53; focused by the press
    assert_eq!(
        *page.log.borrow(),
        [
            "scrub input \"55\" 55",
            "scrub input \"53\" 53",
            "scrub change \"53\" 53"
        ]
    );
    assert_eq!(page.doc.borrow().active_element(), Some(page.scrub));
}

#[test]
fn a_press_without_movement_is_a_click_and_fires_no_change() {
    // Given
    let page = page();

    // When
    Document::pointer_pressed(&page.doc, [50.0, 36.0]);
    Document::pointer_released(&page.doc, [50.0, 36.0]);

    // Then
    assert!(page.log.borrow().is_empty());
    assert_eq!(value(&page, page.scrub), "50");
}

#[test]
fn the_attribute_names_the_px_per_step_and_the_range_clamps() {
    // Given: 10 px per step
    let page = page();
    page.doc
        .borrow_mut()
        .set_attribute(page.scrub, "data-scrub", "10");
    page.doc
        .borrow_mut()
        .update(support::viewport(400.0, 100.0))
        .expect("updates");

    // When: 25 px right is 2 steps; 1000 px is clamped at 100
    Document::pointer_pressed(&page.doc, [50.0, 36.0]);
    Document::pointer_moved(&page.doc, [75.0, 36.0]);
    Document::pointer_moved(&page.doc, [1050.0, 36.0]);
    Document::pointer_released(&page.doc, [1050.0, 36.0]);

    // Then
    assert_eq!(
        *page.log.borrow(),
        [
            "scrub input \"52\" 52",
            "scrub input \"100\" 100",
            "scrub change \"100\" 100"
        ]
    );
}

#[test]
fn a_field_without_the_attribute_does_not_scrub() {
    // Given: `amount` has no data-scrub
    let page = page();

    // When
    Document::pointer_pressed(&page.doc, [50.0, 12.0]);
    Document::pointer_moved(&page.doc, [90.0, 12.0]);
    Document::pointer_released(&page.doc, [90.0, 12.0]);

    // Then
    assert!(page.log.borrow().is_empty());
}

#[test]
fn a_finger_along_the_field_scrubs_and_one_across_is_the_pans() {
    // Given
    let page = page();

    // When: finger 0 along (owned), lifted; finger 1 across
    Document::touch_pressed(&page.doc, 0, [50.0, 36.0]);
    Document::touch_moved(&page.doc, 0, [60.0, 38.0]);
    let along_owned = page.doc.borrow().touch_owned(0);
    Document::touch_released(&page.doc, 0, [60.0, 38.0]);
    Document::touch_pressed(&page.doc, 1, [50.0, 36.0]);
    Document::touch_moved(&page.doc, 1, [54.0, 70.0]);
    let across_owned = page.doc.borrow().touch_owned(1);
    Document::touch_released(&page.doc, 1, [54.0, 70.0]);

    // Then
    assert!(along_owned && !across_owned);
    assert_eq!(
        *page.log.borrow(),
        ["scrub input \"55\" 55", "scrub change \"55\" 55"]
    );
}

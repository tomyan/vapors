//! review-v220's cases, each a listener that removes its control while
//! a default action or a follow-on still holds it: Space's keyup on a
//! button, a number field's commit on blur, and a range, scrub or
//! text drag's next move.

mod support;

use support::sweep_page::{Page, Removed, page, remove_on, the_viewport};
use vapors_dom::{Dom, Event};
use vapors_elements::{Document, Modifiers};

const BUTTON: usize = 0;
const TEXT: usize = 2;
const NUMBER: usize = 3;
const SCRUB: usize = 4;
const RANGE: usize = 5;

fn still_updates(page: &Page) {
    assert!(!page.doc.borrow().contains(page.control));
    page.doc
        .borrow_mut()
        .update(the_viewport())
        .expect("the document updates after the removal");
}

fn drag_from_press(page: &Page) {
    let [x, y] = page.point;
    Document::pointer_pressed(&page.doc, page.point);
    Document::pointer_moved(&page.doc, [x + 20.0, y]);
    Document::pointer_moved(&page.doc, [x + 40.0, y]);
    Document::pointer_released(&page.doc, [x + 40.0, y]);
}

#[test]
fn a_space_keyup_that_removes_the_button_after_a_prevented_keydown_clicks_nothing() {
    // Given: the keydown prevented, so no Space press is recorded
    let page = page(BUTTON);
    Document::focus(&page.doc, Some(page.control));
    page.doc.borrow_mut().listen(
        page.control,
        "keydown",
        Box::new(|e: &mut Event| e.prevent_default()),
    );
    remove_on(&page, "keyup", Removed::Target);

    // When
    Document::key_pressed(&page.doc, " ", Modifiers::default());
    Document::key_released(&page.doc, " ", Modifiers::default());

    // Then
    still_updates(&page);
}

#[test]
fn a_space_keyup_with_no_keydown_that_removes_the_button_clicks_nothing() {
    // Given
    let page = page(BUTTON);
    Document::focus(&page.doc, Some(page.control));
    remove_on(&page, "keyup", Removed::Target);

    // When
    Document::key_released(&page.doc, " ", Modifiers::default());

    // Then
    still_updates(&page);
}

#[test]
fn a_number_fields_change_on_blur_that_removes_it_ends_the_commit() {
    // Given: tapped, "5" typed, its change removing it
    let page = page(NUMBER);
    Document::tapped(&page.doc, page.point);
    Document::key_pressed(&page.doc, "5", Modifiers::default());
    Document::character_typed(&page.doc, "5");
    remove_on(&page, "change", Removed::Target);

    // When
    Document::key_pressed(&page.doc, "Tab", Modifiers::default());

    // Then
    still_updates(&page);
}

#[test]
fn a_pointermove_that_removes_the_dragged_range_ends_the_drag() {
    // Given
    let page = page(RANGE);
    remove_on(&page, "pointermove", Removed::Target);

    // When
    drag_from_press(&page);

    // Then
    still_updates(&page);
}

#[test]
fn a_touch_move_that_removes_the_dragged_range_ends_the_drag() {
    // Given
    let page = page(RANGE);
    remove_on(&page, "pointermove", Removed::Target);
    let [x, y] = page.point;

    // When
    Document::touch_pressed(&page.doc, 1, page.point);
    Document::touch_moved(&page.doc, 1, [x + 20.0, y]);
    Document::touch_moved(&page.doc, 1, [x + 40.0, y]);
    Document::touch_released(&page.doc, 1, [x + 40.0, y]);

    // Then
    still_updates(&page);
}

#[test]
fn a_ranges_input_that_removes_it_ends_the_drag_before_the_next_move() {
    // Given
    let page = page(RANGE);
    remove_on(&page, "input", Removed::Target);

    // When
    drag_from_press(&page);

    // Then
    still_updates(&page);
}

#[test]
fn a_pointermove_that_removes_the_scrubbed_number_ends_the_scrub() {
    // Given
    let page = page(SCRUB);
    remove_on(&page, "pointermove", Removed::Target);

    // When
    drag_from_press(&page);

    // Then
    still_updates(&page);
}

#[test]
fn a_pointermove_that_removes_the_text_field_ends_the_drag_select() {
    // Given
    let page = page(TEXT);
    remove_on(&page, "pointermove", Removed::Target);

    // When
    drag_from_press(&page);

    // Then
    still_updates(&page);
}

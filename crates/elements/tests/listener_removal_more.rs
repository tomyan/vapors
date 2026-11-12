//! More listeners that remove their element (review-fable-input H1), on
//! the paths `listener_removal.rs` does not reach: a touch's
//! `pointerdown`, a number field's step, and a text field's caret
//! after a press, a click or a tap.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::number_page::{key as number_key, page as number_page};
use support::range_page::page as range_page;
use support::text_page::{click, control, key_with, page as text_page, selection, value};
use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::Document;

/// `event` on `node` removes `node`.
fn remove_on(doc: &Rc<RefCell<Document>>, node: NodeId, event: &str) {
    let d = Rc::clone(doc);
    doc.borrow_mut().listen(
        node,
        event,
        Box::new(move |_: &mut Event| {
            if d.borrow().contains(node) {
                d.borrow_mut().remove(node);
            }
        }),
    );
}

#[test]
fn a_touch_pointerdown_handler_that_removes_the_range_starts_no_drag() {
    // Given
    let page = range_page();
    remove_on(&page.doc, page.level, "pointerdown");

    // When
    Document::touch_pressed(&page.doc, 0, [100.0, 10.0]);
    Document::touch_moved(&page.doc, 0, [120.0, 10.0]);
    Document::touch_released(&page.doc, 0, [120.0, 10.0]);

    // Then: no value was taken from the touch
    assert!(!page.doc.borrow().contains(page.level));
    assert!(page.log.borrow().is_empty(), "{:?}", page.log.borrow());
}

#[test]
fn a_number_input_handler_that_removes_the_field_on_a_step_fires_no_change() {
    // Given: the field focused by a click, its `input` removing it
    let page = number_page();
    Document::tapped(&page.doc, [50.0, 12.0]);
    page.log.borrow_mut().clear();
    remove_on(&page.doc, page.amount, "input");

    // When
    number_key(&page, "ArrowUp");

    // Then
    assert_eq!(page.log.borrow().len(), 1, "{:?}", page.log.borrow());
    assert!(!page.doc.borrow().contains(page.amount));
}

#[test]
fn a_blur_handler_that_removes_the_pressed_field_places_no_caret() {
    // Given: "Warm" focused; its blur removes the empty field below
    let page = text_page();
    click(&page, [1.0, 12.0]);
    remove_on(&page.doc, page.plain, "blur");
    let (d, plain) = (Rc::clone(&page.doc), page.plain);
    page.doc.borrow_mut().listen(
        page.name,
        "blur",
        Box::new(move |_: &mut Event| d.borrow_mut().remove(plain)),
    );

    // When: a press on the empty field
    click(&page, [1.0, 36.0]);

    // Then
    assert!(!page.doc.borrow().contains(page.plain));
    assert_eq!(page.doc.borrow().active_element(), None);
}

#[test]
fn a_click_handler_that_removes_a_field_pressed_in_its_selection_moves_no_caret() {
    // Given: "Warm" selected whole; its click removes it
    let page = text_page();
    click(&page, [1.0, 12.0]);
    key_with(&page, "a", control());
    assert_eq!(selection(&page, page.name), (0, 4));
    remove_on(&page.doc, page.name, "click");

    // When: a press inside the selection, which places the caret on
    // release, after the click
    click(&page, [10.0, 12.0]);

    // Then
    assert!(!page.doc.borrow().contains(page.name));
}

#[test]
fn a_tap_click_handler_that_removes_the_field_places_no_caret() {
    // Given
    let page = text_page();
    remove_on(&page.doc, page.name, "click");

    // When
    Document::touch_pressed(&page.doc, 0, [10.0, 12.0]);
    Document::touch_released(&page.doc, 0, [10.0, 12.0]);
    Document::touch_tapped(&page.doc, [10.0, 12.0]);

    // Then
    assert!(!page.doc.borrow().contains(page.name));
    assert_eq!(value(&page, page.plain), "");
}

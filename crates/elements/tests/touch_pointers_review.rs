//! Touch cases from review-v78: an element removed under a finger (on
//! the lift, with the implicit capture pending, and on the scroll that
//! cancels it), `:active` with two fingers, and a finger that lands on
//! nothing.

mod support;

use std::rc::Rc;

use support::pointer::{Log, record, setup, update};
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_scene::Item;

const SHEET: &str = "body { margin: 0; } .a, .b { height: 20px; } .a:active { background-color: #00ff00; } .b:active { background-color: #0000ff; }";

fn active_colours(doc: &Rc<std::cell::RefCell<Document>>) -> Vec<[u8; 4]> {
    update(doc);
    doc.borrow()
        .painted()
        .expect("painted")
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Quad(quad) => Some(quad.background.to_rgba8()),
            _ => None,
        })
        .filter(|rgba| *rgba != [0, 0, 0, 0])
        .collect()
}

#[test]
fn a_finger_lifting_off_a_removed_element_is_quiet() {
    // Given: down on .a, moved (the capture taken), .a removed
    let (doc, _row, a, _b) = setup(SHEET);
    Document::touch_pressed(&doc, 1, [5.0, 5.0]);
    Document::touch_moved(&doc, 1, [5.0, 6.0]);
    doc.borrow_mut().remove(a);
    update(&doc);

    // When / Then: no panic
    Document::touch_released(&doc, 1, [5.0, 6.0]);
}

#[test]
fn a_finger_lifting_with_its_capture_pending_on_a_removed_element_is_quiet() {
    // Given: down on .a (the capture still pending), .a removed
    let (doc, _row, a, _b) = setup(SHEET);
    Document::touch_pressed(&doc, 1, [5.0, 5.0]);
    doc.borrow_mut().remove(a);
    update(&doc);

    // When / Then: no panic
    Document::touch_released(&doc, 1, [5.0, 5.0]);
}

#[test]
fn a_scroll_cancelling_a_finger_on_a_removed_element_is_quiet() {
    // Given
    let (doc, _row, a, _b) = setup(SHEET);
    Document::touch_pressed(&doc, 1, [5.0, 5.0]);
    Document::touch_moved(&doc, 1, [5.0, 6.0]);
    doc.borrow_mut().remove(a);
    update(&doc);

    // When
    Document::cancel_unowned_touches(&doc);

    // Then: the finger is forgotten
    assert!(!doc.borrow().any_touch_owned());
    assert_eq!(active_colours(&doc), Vec::<[u8; 4]>::new());
}

#[test]
fn active_follows_the_first_finger_down_only() {
    // Given: finger 1 on .a, then finger 2 on .b
    let (doc, _row, _a, _b) = setup(SHEET);
    Document::touch_pressed(&doc, 1, [5.0, 5.0]);
    Document::touch_pressed(&doc, 2, [5.0, 30.0]);
    assert_eq!(
        active_colours(&doc),
        [[0, 255, 0, 255]],
        "the first finger's"
    );

    // When: the first lifts
    Document::touch_released(&doc, 1, [5.0, 5.0]);

    // Then: nothing is active, though finger 2 is still down
    assert_eq!(active_colours(&doc), Vec::<[u8; 4]>::new());
}

#[test]
fn a_finger_that_landed_on_nothing_is_forgotten_when_it_lifts() {
    // Given: a tap below the content, then a press on .a
    let (doc, _row, a, _b) = setup(SHEET);
    let log = Log::default();
    record(&doc, a, "pointerup", "a", &log);
    Document::touch_pressed(&doc, 1, [5.0, 90.0]);
    Document::touch_released(&doc, 1, [5.0, 90.0]);
    Document::touch_pressed(&doc, 2, [5.0, 5.0]);
    assert_eq!(
        active_colours(&doc),
        [[0, 255, 0, 255]],
        "the first finger left, so this one is primary"
    );

    // When
    Document::touch_released(&doc, 2, [5.0, 5.0]);

    // Then: .a is no longer active
    assert_eq!(*log.borrow(), ["a pointerup"]);
    assert_eq!(active_colours(&doc), Vec::<[u8; 4]>::new());
}

//! Touches as pointers (3-E1b): each finger is a pointer (id 2 + its
//! finger number) whose `pointerdown` target captures it without asking
//! (implicit capture), so its moves and its `pointerup` go there; a
//! cancelled touch gets `pointercancel`. A listener that calls
//! `set_pointer_capture` owns the touch, which the app reads before
//! letting a pan scroll.

mod support;

use support::pointer::{Log, SHEET, record, setup};
use vapors_dom::{Dom, Event};
use vapors_elements::Document;

const EVENTS: [&str; 6] = [
    "pointerdown",
    "pointermove",
    "pointerup",
    "pointercancel",
    "gotpointercapture",
    "lostpointercapture",
];

fn logged(
    doc: &std::rc::Rc<std::cell::RefCell<Document>>,
    nodes: &[(vapors_dom::NodeId, &str)],
) -> Log {
    let log = Log::default();
    for (node, label) in nodes {
        for event in EVENTS {
            record(doc, *node, event, label, &log);
        }
    }
    log
}

#[test]
fn a_touch_is_captured_by_where_it_went_down() {
    // Given
    let (doc, _row, a, b) = setup(SHEET);
    let log = logged(&doc, &[(a, "a"), (b, "b")]);

    // When: down on .a, moved over .b, lifted there
    Document::touch_pressed(&doc, 7, [5.0, 5.0]);
    Document::touch_moved(&doc, 7, [5.0, 30.0]);
    Document::touch_released(&doc, 7, [5.0, 30.0]);

    // Then
    assert_eq!(
        *log.borrow(),
        [
            "a pointerdown",
            "a gotpointercapture",
            "a pointermove",
            "a pointerup",
            "a lostpointercapture",
        ]
    );
}

#[test]
fn a_touch_carries_its_own_pointer_id() {
    // Given
    let (doc, _row, a, _b) = setup(SHEET);
    let id = std::rc::Rc::new(std::cell::Cell::new(0));
    let into = std::rc::Rc::clone(&id);
    doc.borrow_mut().listen(
        a,
        "pointerdown",
        Box::new(move |e: &mut Event| into.set(e.pointer().map_or(0, |p| p.id))),
    );

    // When
    Document::touch_pressed(&doc, 7, [5.0, 5.0]);

    // Then
    assert_eq!(id.get(), 9);
}

#[test]
fn a_cancelled_touch_gets_pointercancel_and_nothing_after() {
    // Given
    let (doc, _row, a, b) = setup(SHEET);
    let log = logged(&doc, &[(a, "a"), (b, "b")]);
    Document::touch_pressed(&doc, 1, [5.0, 5.0]);

    // When
    Document::touch_cancelled(&doc, 1, [5.0, 6.0]);
    Document::touch_released(&doc, 1, [5.0, 30.0]);

    // Then: no pointerup after the cancel
    assert_eq!(
        *log.borrow(),
        [
            "a pointerdown",
            "a gotpointercapture",
            "a pointercancel",
            "a lostpointercapture"
        ]
    );
}

#[test]
fn a_touch_is_owned_only_when_a_listener_captured_it() {
    // Given: .b's listener captures; .a's does not
    let (doc, _row, _a, b) = setup(SHEET);
    doc.borrow_mut().listen(
        b,
        "pointerdown",
        Box::new(|e: &mut Event| e.set_pointer_capture()),
    );

    // When / Then
    Document::touch_pressed(&doc, 1, [5.0, 5.0]);
    assert!(
        !doc.borrow().touch_owned(1),
        "an implicit capture does not own it"
    );
    Document::touch_pressed(&doc, 2, [5.0, 30.0]);
    assert!(doc.borrow().touch_owned(2));
    Document::touch_released(&doc, 2, [5.0, 30.0]);
    assert!(
        !doc.borrow().touch_owned(2),
        "a lifted touch is owned by nothing"
    );
}

#[test]
fn two_fingers_are_captured_apart() {
    // Given
    let (doc, _row, a, b) = setup(SHEET);
    let log = Log::default();
    record(&doc, a, "pointermove", "a", &log);
    record(&doc, b, "pointermove", "b", &log);
    Document::touch_pressed(&doc, 1, [5.0, 5.0]);
    Document::touch_pressed(&doc, 2, [5.0, 30.0]);

    // When: each crosses over the other's element
    Document::touch_moved(&doc, 1, [5.0, 30.0]);
    Document::touch_moved(&doc, 2, [5.0, 5.0]);

    // Then
    assert_eq!(*log.borrow(), ["a pointermove", "b pointermove"]);
}

#[test]
fn a_tap_clicks_without_a_second_press() {
    // Given: the touch's own down and up came first
    let (doc, _row, a, _b) = setup(SHEET);
    let log = Log::default();
    for event in ["pointerdown", "pointerup", "click"] {
        record(&doc, a, event, "a", &log);
    }
    Document::touch_pressed(&doc, 1, [5.0, 5.0]);
    Document::touch_released(&doc, 1, [5.0, 5.0]);

    // When: the platform's tap gesture follows
    Document::touch_tapped(&doc, [5.0, 5.0]);

    // Then
    assert_eq!(*log.borrow(), ["a pointerdown", "a pointerup", "a click"]);
}

#[test]
fn a_scroll_cancels_the_touches_no_listener_owns() {
    // Given: finger 1 on .a (implicit capture), finger 2 on .b, which owns it
    let (doc, _row, a, b) = setup(SHEET);
    doc.borrow_mut().listen(
        b,
        "pointerdown",
        Box::new(|e: &mut Event| e.set_pointer_capture()),
    );
    let log = Log::default();
    record(&doc, a, "pointercancel", "a", &log);
    record(&doc, b, "pointercancel", "b", &log);
    Document::touch_pressed(&doc, 1, [5.0, 5.0]);
    Document::touch_pressed(&doc, 2, [5.0, 30.0]);
    assert!(doc.borrow().any_touch_owned());

    // When
    Document::cancel_unowned_touches(&doc);

    // Then
    assert_eq!(*log.borrow(), ["a pointercancel"]);
}

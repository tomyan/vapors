//! Pointer capture (3-E1a): a `pointerdown` listener that calls
//! `set_pointer_capture` makes its element the target of every later
//! `pointermove` and the `pointerup` wherever the pointer is, with the
//! position unrounded in CSS px; the capture ends on `pointerup`, with
//! `gotpointercapture` and `lostpointercapture` around it.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::pointer::{Log, SHEET, record, setup};
use vapors_dom::{Dom, Event};
use vapors_elements::Document;

/// `.a` captures on pointerdown; returns the positions its moves saw.
fn capturing(doc: &Rc<RefCell<Document>>, a: vapors_dom::NodeId) -> Rc<RefCell<Vec<[f32; 2]>>> {
    let seen = Rc::new(RefCell::new(Vec::new()));
    doc.borrow_mut().listen(
        a,
        "pointerdown",
        Box::new(|e: &mut Event| e.set_pointer_capture()),
    );
    let moves = Rc::clone(&seen);
    doc.borrow_mut().listen(
        a,
        "pointermove",
        Box::new(move |e: &mut Event| moves.borrow_mut().push(e.client())),
    );
    seen
}

#[test]
fn a_captured_pointer_moves_its_element_wherever_it_goes() {
    // Given: pressed on .a (y 0..20), which captures
    let (doc, _row, a, _b) = setup(SHEET);
    let seen = capturing(&doc, a);
    Document::pointer_moved(&doc, [5.0, 5.0]);
    Document::pointer_pressed(&doc, [5.0, 5.0]);

    // When: moved over .b, then outside the window's content
    Document::pointer_moved(&doc, [5.25, 30.5]);
    Document::pointer_moved(&doc, [250.75, 400.125]);

    // Then: .a saw every move, unrounded (the first, before the press)
    assert_eq!(
        *seen.borrow(),
        [[5.0, 5.0], [5.25, 30.5], [250.75, 400.125]]
    );
}

#[test]
fn without_capture_moves_go_to_what_is_under_the_pointer() {
    // Given: .a does not capture
    let (doc, _row, a, b) = setup(SHEET);
    let log = Log::default();
    record(&doc, a, "pointermove", "a", &log);
    record(&doc, b, "pointermove", "b", &log);
    Document::pointer_moved(&doc, [5.0, 5.0]);
    Document::pointer_pressed(&doc, [5.0, 5.0]);

    // When
    Document::pointer_moved(&doc, [5.0, 30.0]);

    // Then: the first move went to .a, the second to .b
    assert_eq!(*log.borrow(), ["a pointermove", "b pointermove"]);
}

#[test]
fn the_release_goes_to_the_capturing_element_and_ends_the_capture() {
    // Given
    let (doc, _row, a, b) = setup(SHEET);
    let _seen = capturing(&doc, a);
    let log = Log::default();
    for (node, label) in [(a, "a"), (b, "b")] {
        for event in [
            "pointerup",
            "gotpointercapture",
            "lostpointercapture",
            "click",
            "pointermove",
        ] {
            record(&doc, node, event, label, &log);
        }
    }
    Document::pointer_moved(&doc, [5.0, 5.0]);
    Document::pointer_pressed(&doc, [5.0, 5.0]);

    // When: released over .b, then moved over .b
    Document::pointer_moved(&doc, [5.0, 30.0]);
    Document::pointer_released(&doc, [5.0, 30.0]);
    Document::pointer_moved(&doc, [5.0, 31.0]);

    // Then: the move before the press, then as Chromium orders them;
    // after the release, moves hit-test again
    assert_eq!(
        *log.borrow(),
        [
            "a pointermove",
            "a gotpointercapture",
            "a pointermove",
            "a pointerup",
            "a lostpointercapture",
            "a click",
            "b pointermove",
        ]
    );
}

//! Pointer capture's edges (3-E1a): the offset from the target's
//! padding box, `release_pointer_capture`, the pointer leaving the
//! window while captured, and a capturing element removed mid-drag.

mod support;

use support::pointer::{Log, record, setup};
use vapors_dom::{Dom, Event};
use vapors_elements::Document;

const PADDED: &str = "body { margin: 0; } .a { height: 20px; margin-left: 10px; border-left: 3px solid black; } .b { height: 20px; }";

#[test]
fn the_offset_is_from_the_targets_padding_box() {
    // Given: .a's padding box starts at x 13 (10 margin, 3 border)
    let (doc, _row, a, _b) = setup(PADDED);
    let seen = std::rc::Rc::new(std::cell::Cell::new([0.0_f32; 2]));
    let into = std::rc::Rc::clone(&seen);
    doc.borrow_mut().listen(
        a,
        "pointerdown",
        Box::new(move |e: &mut Event| into.set(e.offset())),
    );

    // When
    Document::pointer_moved(&doc, [20.5, 4.25]);
    Document::pointer_pressed(&doc, [20.5, 4.25]);

    // Then
    assert_eq!(seen.get(), [7.5, 4.25]);
}

#[test]
fn release_pointer_capture_sends_moves_to_what_is_under_the_pointer_again() {
    // Given: .a captures on pointerdown and releases on its first move
    let (doc, _row, a, b) = setup(PADDED);
    doc.borrow_mut().listen(
        a,
        "pointerdown",
        Box::new(|e: &mut Event| e.set_pointer_capture()),
    );
    doc.borrow_mut().listen(
        a,
        "pointermove",
        Box::new(|e: &mut Event| e.release_pointer_capture()),
    );
    let log = Log::default();
    record(&doc, a, "lostpointercapture", "a", &log);
    record(&doc, b, "pointermove", "b", &log);
    Document::pointer_moved(&doc, [15.0, 5.0]);
    Document::pointer_pressed(&doc, [15.0, 5.0]);

    // When: two moves over .b
    Document::pointer_moved(&doc, [5.0, 30.0]);
    Document::pointer_moved(&doc, [5.0, 31.0]);

    // Then: the first still went to .a, which released; the second to .b
    assert_eq!(*log.borrow(), ["a lostpointercapture", "b pointermove"]);
}

#[test]
fn leaving_the_window_while_captured_keeps_the_press() {
    // Given
    let (doc, _row, a, _b) = setup(PADDED);
    doc.borrow_mut().listen(
        a,
        "pointerdown",
        Box::new(|e: &mut Event| e.set_pointer_capture()),
    );
    let log = Log::default();
    record(&doc, a, "pointerup", "a", &log);
    record(&doc, a, "click", "a", &log);
    Document::pointer_moved(&doc, [15.0, 5.0]);
    Document::pointer_pressed(&doc, [15.0, 5.0]);
    Document::pointer_moved(&doc, [15.0, 6.0]);

    // When: out of the window and released there
    Document::pointer_left(&doc);
    Document::pointer_released(&doc, [-40.0, -3.0]);

    // Then: the release and the click still reach .a
    assert_eq!(*log.borrow(), ["a pointerup", "a click"]);
}

#[test]
fn a_capturing_element_removed_mid_drag_gives_the_pointer_back() {
    // Given: .a captures, then is removed
    let (doc, _row, a, b) = setup(PADDED);
    doc.borrow_mut().listen(
        a,
        "pointerdown",
        Box::new(|e: &mut Event| e.set_pointer_capture()),
    );
    let log = Log::default();
    record(&doc, b, "pointermove", "b", &log);
    Document::pointer_moved(&doc, [15.0, 5.0]);
    Document::pointer_pressed(&doc, [15.0, 5.0]);
    Document::pointer_moved(&doc, [15.0, 6.0]);
    doc.borrow_mut().remove(a);
    support::pointer::update(&doc);

    // When: over .b, which moved up to y 0..20
    Document::pointer_moved(&doc, [5.0, 10.0]);

    // Then
    assert_eq!(*log.borrow(), ["b pointermove"]);
}

#[test]
fn capture_moving_between_elements_is_lost_by_one_then_got_by_the_other() {
    // Given: .a captures on pointerdown; .row takes it on the first move,
    // as the move bubbles from .a
    let (doc, row, a, _b) = setup(PADDED);
    doc.borrow_mut().listen(
        a,
        "pointerdown",
        Box::new(|e: &mut Event| e.set_pointer_capture()),
    );
    doc.borrow_mut().listen(
        row,
        "pointermove",
        Box::new(|e: &mut Event| e.set_pointer_capture()),
    );
    let log = Log::default();
    for (node, label) in [(a, "a"), (row, "row")] {
        record(&doc, node, "lostpointercapture", label, &log);
        record(&doc, node, "gotpointercapture", label, &log);
    }
    Document::pointer_moved(&doc, [15.0, 5.0]);
    Document::pointer_pressed(&doc, [15.0, 5.0]);
    Document::pointer_moved(&doc, [15.0, 6.0]);
    log.borrow_mut().clear();

    // When
    Document::pointer_moved(&doc, [15.0, 7.0]);

    // Then: both bubble, as in the DOM, so .row also hears .a's loss
    assert_eq!(
        *log.borrow(),
        [
            "a lostpointercapture",
            "row lostpointercapture",
            "row gotpointercapture"
        ]
    );
}

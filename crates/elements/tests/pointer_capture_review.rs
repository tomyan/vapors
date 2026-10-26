//! Pointer capture's cases from review-v77: a capturing element removed
//! before the release, `:hover` while captured, capture asked for with
//! no active pointer, and a second `pointerdown` while captured.

mod support;

use std::panic::{AssertUnwindSafe, catch_unwind};

use support::pointer::{Log, record, setup, update};
use vapors_dom::{Dom, Event};
use vapors_elements::Document;
use vapors_scene::Item;

const SHEET: &str = "body { margin: 0; } .a, .b { height: 20px; } .a:hover { background-color: #00ff00; } .b:hover { background-color: #0000ff; }";

fn captures(doc: &std::rc::Rc<std::cell::RefCell<Document>>, node: vapors_dom::NodeId) {
    doc.borrow_mut().listen(
        node,
        "pointerdown",
        Box::new(|e: &mut Event| e.set_pointer_capture()),
    );
}

#[test]
fn releasing_after_the_capturing_element_was_removed_sends_lostpointercapture_nowhere() {
    // Given: .a captures, then is removed
    let (doc, row, a, _b) = setup(SHEET);
    captures(&doc, a);
    let log = Log::default();
    record(&doc, row, "lostpointercapture", "row", &log);
    Document::pointer_moved(&doc, [5.0, 5.0]);
    Document::pointer_pressed(&doc, [5.0, 5.0]);
    Document::pointer_moved(&doc, [5.0, 6.0]);
    doc.borrow_mut().remove(a);

    // When: released, and left
    Document::pointer_released(&doc, [5.0, 6.0]);
    Document::pointer_left(&doc);

    // Then: no panic, and the loss went nowhere (the web fires it on the
    // removed element, which nothing observes)
    assert_eq!(*log.borrow(), Vec::<String>::new());
}

#[test]
fn hover_stays_on_the_capturing_element() {
    // Given: .a captures
    let (doc, _row, a, _b) = setup(SHEET);
    captures(&doc, a);
    Document::pointer_moved(&doc, [5.0, 5.0]);
    Document::pointer_pressed(&doc, [5.0, 5.0]);

    // When: over .b
    Document::pointer_moved(&doc, [5.0, 30.0]);
    update(&doc);

    // Then: .a is drawn hovered, .b is not
    let colours: Vec<[u8; 4]> = doc
        .borrow()
        .painted()
        .expect("painted")
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Quad(quad) => Some(quad.background.to_rgba8()),
            _ => None,
        })
        .collect();
    assert!(colours.contains(&[0, 255, 0, 255]), "{colours:?}");
    assert!(!colours.contains(&[0, 0, 255, 255]), "{colours:?}");
}

#[test]
fn capture_asked_for_in_a_click_listener_is_refused_by_name() {
    // Given: .a asks for capture on click, after the pointer was released
    let (doc, _row, a, _b) = setup(SHEET);
    doc.borrow_mut().listen(
        a,
        "click",
        Box::new(|e: &mut Event| e.set_pointer_capture()),
    );
    Document::pointer_moved(&doc, [5.0, 5.0]);
    Document::pointer_pressed(&doc, [5.0, 5.0]);

    // When
    let refused = catch_unwind(AssertUnwindSafe(|| {
        Document::pointer_released(&doc, [5.0, 5.0]);
    }));

    // Then: as the web throws InvalidStateError
    let message = refused.expect_err("refused");
    let message = message
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_default();
    assert!(
        message.contains("`click`") && message.contains("no pressed pointer"),
        "{message}"
    );
}

#[test]
fn a_pointerdown_while_captured_goes_to_the_capturing_element() {
    // Given: .a captures; the pointer is over .b
    let (doc, _row, a, b) = setup(SHEET);
    captures(&doc, a);
    let log = Log::default();
    record(&doc, a, "pointerdown", "a", &log);
    record(&doc, b, "pointerdown", "b", &log);
    Document::pointer_moved(&doc, [5.0, 5.0]);
    Document::pointer_pressed(&doc, [5.0, 5.0]);
    Document::pointer_moved(&doc, [5.0, 30.0]);

    // When: a second press there (another button, as the platform sends it)
    Document::pointer_pressed(&doc, [5.0, 30.0]);

    // Then
    assert_eq!(*log.borrow(), ["a pointerdown", "a pointerdown"]);
}

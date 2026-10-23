//! Pointer input dispatch: DOM-style bubbling, `stop_propagation`,
//! `click` on the nearest common ancestor, and a disabled button.

mod support;

use support::pointer::{Log, SHEET, record, setup, update};
use vapors_dom::{Dom, Event};
use vapors_elements::Document;

#[test]
fn a_click_reaches_the_target_then_its_ancestors() {
    // Given
    let (doc, row, a, _) = setup(SHEET);
    let log = Log::default();
    record(&doc, a, "click", "a", &log);
    record(&doc, row, "click", "row", &log);

    // When
    Document::pointer_pressed(&doc, [5.0, 5.0]);
    Document::pointer_released(&doc, [5.0, 5.0]);

    // Then
    assert_eq!(*log.borrow(), ["a click", "row click"]);
}

#[test]
fn stop_propagation_keeps_the_event_from_the_ancestors() {
    // Given
    let (doc, row, a, _) = setup(SHEET);
    let log = Log::default();
    doc.borrow_mut()
        .listen(a, "click", Box::new(|e: &mut Event| e.stop_propagation()));
    record(&doc, row, "click", "row", &log);

    // When
    Document::tapped(&doc, [5.0, 5.0]);

    // Then
    assert_eq!(*log.borrow(), Vec::<String>::new());
}

#[test]
fn press_on_one_child_and_release_on_another_clicks_their_parent() {
    // Given
    let (doc, row, a, b) = setup(SHEET);
    let log = Log::default();
    for (node, label) in [(row, "row"), (a, "a"), (b, "b")] {
        record(&doc, node, "click", label, &log);
        record(&doc, node, "pointerup", label, &log);
    }

    // When: down on .a, up on .b
    Document::pointer_pressed(&doc, [5.0, 5.0]);
    Document::pointer_released(&doc, [5.0, 25.0]);

    // Then
    assert_eq!(*log.borrow(), ["b pointerup", "row pointerup", "row click"]);
}

#[test]
fn a_disabled_button_gets_pointerdown_but_no_click() {
    // Given
    let (doc, row, _, _) = setup("body { margin: 0; } .row { display: flex; height: 20px; }");
    let button = {
        let mut d = doc.borrow_mut();
        let button = d.create_element("button");
        d.set_attribute(button, "disabled", "");
        d.append(row, button);
        button
    };
    update(&doc);
    let log = Log::default();
    record(&doc, button, "click", "button", &log);
    record(&doc, row, "click", "row", &log);
    record(&doc, button, "pointerdown", "button", &log);

    // When
    Document::tapped(&doc, [2.0, 2.0]);

    // Then
    assert_eq!(*log.borrow(), ["button pointerdown"]);
}

//! The mouse's click count and modifiers: the second press in a row
//! fires `dblclick` after its `click`, as Chromium does; the modifiers
//! held are on `pointerdown`, `pointerup`, `click` and `dblclick`
//! (the DOM's altKey, ctrlKey, metaKey and shiftKey).

mod support;

use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;

use support::pointer::{Log, SHEET, setup};
use vapors_dom::{Dom, Event};
use vapors_elements::{Document, Modifiers};

fn clicks(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("a count")
}

/// Logs each `names` event on `node` with its alt modifier.
fn log_with_alt(doc: &Rc<RefCell<Document>>, node: vapors_dom::NodeId, names: &[&str]) -> Log {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    for name in names {
        let log = Rc::clone(&log);
        doc.borrow_mut().listen(
            node,
            name,
            Box::new(move |e: &mut Event| {
                let alt = if e.modifiers().alt { " alt" } else { "" };
                log.borrow_mut().push(format!("{}{alt}", e.name()));
            }),
        );
    }
    log
}

fn click(doc: &Rc<RefCell<Document>>, n: u32, modifiers: Modifiers) {
    Document::pointer_pressed_with(doc, [5.0, 5.0], clicks(n), modifiers);
    Document::pointer_released_with(doc, [5.0, 5.0], modifiers);
}

#[test]
fn the_second_press_in_a_row_fires_dblclick_after_its_click() {
    // Given
    let (doc, _, a, _) = setup(SHEET);
    let log = log_with_alt(&doc, a, &["click", "dblclick"]);

    // When
    click(&doc, 1, Modifiers::default());
    click(&doc, 2, Modifiers::default());

    // Then
    assert_eq!(*log.borrow(), ["click", "click", "dblclick"]);
}

#[test]
fn the_modifiers_held_are_on_the_pointer_events_and_the_clicks() {
    // Given
    let (doc, _, a, _) = setup(SHEET);
    let log = log_with_alt(&doc, a, &["pointerdown", "pointerup", "click", "dblclick"]);
    let alt = Modifiers {
        alt: true,
        ..Modifiers::default()
    };

    // When
    click(&doc, 1, Modifiers::default());
    click(&doc, 2, alt);

    // Then
    assert_eq!(
        *log.borrow(),
        [
            "pointerdown",
            "pointerup",
            "click",
            "pointerdown alt",
            "pointerup alt",
            "click alt",
            "dblclick alt"
        ]
    );
}

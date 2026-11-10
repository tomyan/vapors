use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_dom::{Dom, Event as DomEvent};

use vapors_platform::{Event, Instant};

use super::tests::{give, key, page, press, shown};
use crate::DocumentInput;

fn observe(input: &mut DocumentInput, doc: &RefCell<vapors_elements::Document>, event: &Event) {
    input
        .observe(&doc.borrow(), event, Instant::now())
        .expect("placed");
}

#[test]
fn a_key_observed_hides_a_shown_pointer_tooltip() {
    // Given
    let (doc, _) = page();
    let mut input = DocumentInput::new();
    observe(&mut input, &doc, &Event::PointerMoved { x: 10.0, y: 10.0 });
    let deadline = input.tooltip_deadline().expect("armed");
    assert_eq!(shown(&mut input, &doc, deadline), Some("Open (o)".into()));

    // When
    observe(&mut input, &doc, &key("o", "KeyO"));

    // Then
    assert_eq!(shown(&mut input, &doc, deadline), None);
    assert_eq!(input.tooltip_deadline(), None);
}

#[test]
fn a_tab_observed_with_nothing_focused_focuses_nothing() {
    // Given
    let (doc, _) = page();
    let mut input = DocumentInput::new();

    // When
    observe(&mut input, &doc, &key("Tab", "Tab"));

    // Then
    assert_eq!(doc.borrow().active_element(), None);
    assert_eq!(input.tooltip_deadline(), None);
}

#[test]
fn a_move_observed_arms_the_title_under_it_without_hovering() {
    // Given
    let (doc, open) = page();
    let mut input = DocumentInput::new();
    let moves = Rc::new(RefCell::new(0));
    let counter = Rc::clone(&moves);
    doc.borrow_mut().listen(
        open,
        "pointermove",
        Box::new(move |_: &mut DomEvent| *counter.borrow_mut() += 1),
    );

    // When
    observe(&mut input, &doc, &Event::PointerMoved { x: 10.0, y: 10.0 });

    // Then
    let deadline = input.tooltip_deadline().expect("armed");
    let tip = input.tooltip(&doc.borrow(), deadline).cloned();
    assert_eq!(tip.map(|tip| tip.node), Some(open));
    assert_eq!(*moves.borrow(), 0, "no pointermove dispatched");
}

/// The tooltip as a host sees it after each event: the title shown at
/// its deadline (or none armed) and its hotspot.
fn seen(
    input: &mut DocumentInput,
    doc: &RefCell<vapors_elements::Document>,
) -> (Option<String>, Option<vapors_elements::Point>) {
    let at = input
        .tooltip_deadline()
        .map(|at| at + Duration::from_millis(1));
    let text = at.and_then(|at| input.tooltip(&doc.borrow(), at).map(|tip| tip.text.clone()));
    (text, input.tooltip_hotspot())
}

#[test]
fn observe_and_give_leave_the_tooltip_in_the_same_state() {
    // Given: events that move no focus when given
    let events = [
        Event::PointerMoved { x: 10.0, y: 10.0 },
        Event::PointerMoved { x: 150.0, y: 10.0 },
        Event::PointerMoved { x: 20.0, y: 12.0 },
        press(true),
        press(false),
        Event::PointerMoved { x: 250.0, y: 10.0 },
        Event::PointerLeft,
        Event::PointerMoved { x: 30.0, y: 5.0 },
        key("Escape", "Escape"),
        Event::PointerMoved { x: 40.0, y: 5.0 },
        Event::WindowFocus { focused: false },
        Event::PointerMoved { x: 10.0, y: 10.0 },
    ];
    let (given_doc, _) = page();
    let (observed_doc, _) = page();
    let mut given = DocumentInput::new();
    let mut observed = DocumentInput::new();

    for event in &events {
        // When
        give(&mut given, &given_doc, event);
        observe(&mut observed, &observed_doc, event);

        // Then
        let expected = seen(&mut given, &given_doc);
        assert_eq!(seen(&mut observed, &observed_doc), expected, "{event:?}");
    }
}

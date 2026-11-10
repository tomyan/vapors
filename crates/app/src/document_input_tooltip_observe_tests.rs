use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_dom::{Dom, Event as DomEvent};

use vapors_elements::Document;
use vapors_platform::{Event, Instant, TouchPhase};

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
/// its deadline (or none armed), its hotspot, and the pointer.
type Seen = (
    Option<String>,
    Option<vapors_elements::Point>,
    Option<vapors_elements::Point>,
);

fn seen(input: &mut DocumentInput, doc: &RefCell<vapors_elements::Document>) -> Seen {
    let at = input
        .tooltip_deadline()
        .map(|at| at + Duration::from_millis(1));
    let text = at.and_then(|at| input.tooltip(&doc.borrow(), at).map(|tip| tip.text.clone()));
    (text, input.tooltip_hotspot(), input.pointer())
}

/// Two copies of the page, `prepare`d alike; `events` given to one and
/// observed on the other leave the tooltip the same after each.
fn assert_observe_matches_give(prepare: impl Fn(&Rc<RefCell<Document>>), events: &[Event]) {
    let (given_doc, _) = page();
    let (observed_doc, _) = page();
    prepare(&given_doc);
    prepare(&observed_doc);
    let mut given = DocumentInput::new();
    let mut observed = DocumentInput::new();
    for event in events {
        give(&mut given, &given_doc, event);
        observe(&mut observed, &observed_doc, event);
        let expected = seen(&mut given, &given_doc);
        assert_eq!(seen(&mut observed, &observed_doc), expected, "{event:?}");
    }
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

    // When, Then
    assert_observe_matches_give(|_| {}, &events);
}

#[test]
fn observe_and_give_agree_on_events_the_host_takes() {
    // Given: below the buttons nothing is hit, so give routes the wheel
    // and a press there to the host; the wheel hides the keyboard
    // tooltip a key armed, and a move with the button held arms nothing
    let events = [
        key("Escape", "Escape"),
        Event::PointerMoved { x: 10.0, y: 50.0 },
        Event::Scroll { dx: 0.0, dy: 40.0 },
        press(true),
        Event::PointerMoved { x: 10.0, y: 10.0 },
        press(false),
        Event::PointerMoved { x: 150.0, y: 10.0 },
        Event::PointerMoved { x: 10.0, y: 10.0 },
    ];

    // When, Then
    assert_observe_matches_give(focus_name, &events);
}

#[test]
fn observe_and_give_agree_on_the_wheel_and_touch() {
    // Given: each hides a tooltip armed over the titled button
    let events = [
        Event::PointerMoved { x: 10.0, y: 10.0 },
        Event::Scroll { dx: 0.0, dy: 40.0 },
        Event::PointerMoved { x: 150.0, y: 10.0 },
        Event::PointerMoved { x: 10.0, y: 10.0 },
        touch(TouchPhase::Started, 10.0),
        touch(TouchPhase::Moved, 12.0),
        touch(TouchPhase::Ended, 12.0),
        Event::PointerMoved { x: 150.0, y: 10.0 },
        Event::PointerMoved { x: 10.0, y: 10.0 },
        touch(TouchPhase::Started, 60.0),
        touch(TouchPhase::Cancelled, 60.0),
    ];

    // When, Then
    assert_observe_matches_give(|_| {}, &events);
}

#[test]
fn observe_and_give_agree_on_a_key_after_a_script_focused_a_title() {
    // Given: a key arms the keyboard tooltip of the field a script
    // focused
    let events = [
        key("Escape", "Escape"),
        Event::PointerMoved { x: 10.0, y: 10.0 },
        key("Escape", "Escape"),
    ];

    // When, Then
    assert_observe_matches_give(focus_name, &events);
}

/// A script focuses the titled text field, which is then
/// `:focus-visible`.
fn focus_name(doc: &Rc<RefCell<Document>>) {
    let name = {
        let doc = doc.borrow();
        doc.children(doc.body())[2]
    };
    Document::focus(doc, Some(name));
    assert!(doc.borrow().focus_visible());
}

fn touch(phase: TouchPhase, y: f32) -> Event {
    Event::Touch {
        id: 1,
        phase,
        x: 10.0,
        y,
    }
}

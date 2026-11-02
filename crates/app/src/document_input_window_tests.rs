//! The window's focus goes to the document and the host alike: losing
//! it blurs the card's focused control, regaining it focuses it again.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, Event as DomEvent};
use vapors_elements::Document;
use vapors_platform::{Event, Modifiers, Platform, TouchPhase};

use super::Route;
use super::key_tests::{card, click, give, key};
use crate::DocumentInput;

#[test]
fn the_window_losing_and_regaining_focus_blurs_and_refocuses_the_card_for_both() {
    // Given: the card's range focused, its focus events logged
    let (doc, _, range) = card();
    Document::focus(&doc, Some(range));
    let log = Rc::new(RefCell::new(Vec::new()));
    for name in ["blur", "focus"] {
        let log = Rc::clone(&log);
        doc.borrow_mut().listen(
            range,
            name,
            Box::new(move |e: &mut DomEvent| log.borrow_mut().push(e.name().to_owned())),
        );
    }
    let mut input = DocumentInput::new();

    // When
    let routes = [false, true].map(|focused| {
        input
            .give(&doc, &Event::WindowFocus { focused }, &Platform::detached())
            .expect("placed")
            .route
    });

    // Then
    assert_eq!(routes, [Route::Both, Route::Both]);
    assert_eq!(*log.borrow(), ["blur", "focus"]);
    assert_eq!(doc.borrow().active_element(), Some(range));
}

fn window(input: &mut DocumentInput, doc: &Rc<RefCell<Document>>, focused: bool) {
    give(input, doc, &Event::WindowFocus { focused });
}

fn button(pressed: bool) -> Event {
    Event::PrimaryButton {
        pressed,
        clicks: std::num::NonZeroU32::MIN,
        modifiers: Modifiers::default(),
    }
}

#[test]
fn a_key_held_across_the_loss_is_forgotten_so_its_late_release_goes_by_focus() {
    // Given: the slider focused, ArrowRight down on it
    let (doc, _, range) = card();
    let mut input = DocumentInput::new();
    Document::focus(&doc, Some(range));
    give(&mut input, &doc, &key("ArrowRight", Modifiers::default()));

    // When: the window lost and back, a click on the photo, then the up
    window(&mut input, &doc, false);
    window(&mut input, &doc, true);
    click(&mut input, &doc, 150.0, 80.0);
    let up = Event::Key {
        key: "ArrowRight".into(),
        code: "ArrowRight".into(),
        pressed: false,
        repeat: false,
        modifiers: Modifiers::default(),
    };
    let (route, _) = give(&mut input, &doc, &up);

    // Then: by focus, which left the card, not by the stale key-down
    assert_eq!(route, Route::Host);
}

#[test]
fn a_slider_drag_across_the_loss_is_cancelled_and_its_late_release_goes_by_hit() {
    // Given: a press on the slider, logging its pointercancel
    let (doc, _, range) = card();
    let cancels = Rc::new(RefCell::new(0));
    let counted = Rc::clone(&cancels);
    doc.borrow_mut().listen(
        range,
        "pointercancel",
        Box::new(move |_: &mut DomEvent| *counted.borrow_mut() += 1),
    );
    let mut input = DocumentInput::new();
    give(&mut input, &doc, &Event::PointerMoved { x: 80.0, y: 10.0 });
    give(&mut input, &doc, &button(true));
    let pressed_at = doc.borrow().value(range);

    // When: lost and back, then a move over the photo and the release
    window(&mut input, &doc, false);
    window(&mut input, &doc, true);
    give(&mut input, &doc, &Event::PointerMoved { x: 150.0, y: 80.0 });
    let (route, _) = give(&mut input, &doc, &button(false));

    // Then
    assert_eq!(*cancels.borrow(), 1);
    assert_eq!(
        doc.borrow().value(range),
        pressed_at,
        "no drag after the return"
    );
    assert_eq!(route, Route::Host);
}

#[test]
fn a_touch_held_across_the_loss_is_forgotten_so_its_late_lift_goes_by_hit() {
    // Given: a finger down on the card
    let (doc, _, _) = card();
    let mut input = DocumentInput::new();
    let touch = |phase, x, y| Event::Touch { id: 1, phase, x, y };
    give(&mut input, &doc, &touch(TouchPhase::Started, 80.0, 10.0));

    // When: lost and back, then the lift over the photo
    window(&mut input, &doc, false);
    window(&mut input, &doc, true);
    let (route, _) = give(&mut input, &doc, &touch(TouchPhase::Ended, 150.0, 80.0));

    // Then
    assert_eq!(route, Route::Host);
}

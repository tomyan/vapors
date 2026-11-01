//! A key's route held from its down to its up: by its code, so Shift
//! changing its name mid-hold changes nothing; and its up to both sides
//! when the document had its down and did not use it.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;
use vapors_platform::{Event, Modifiers};

use super::Route;
use super::key_tests::{card, click, give};
use crate::DocumentInput;

/// The keyups the range hears.
fn keyups(doc: &Rc<RefCell<Document>>, range: NodeId) -> Rc<RefCell<Vec<String>>> {
    let heard = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&heard);
    doc.borrow_mut().listen(
        range,
        "keyup",
        Box::new(move |e: &mut vapors_dom::Event| log.borrow_mut().push(e.key().to_owned())),
    );
    heard
}

fn press(key: &str, code: &str, pressed: bool, shift: bool) -> Event {
    let modifiers = Modifiers {
        shift,
        ..Modifiers::default()
    };
    Event::Key {
        key: key.into(),
        code: code.into(),
        pressed,
        repeat: false,
        modifiers,
    }
}

#[test]
fn space_unused_by_the_focused_slider_reaches_both_on_its_way_up() {
    // Given
    let (doc, _, range) = card();
    let heard = keyups(&doc, range);
    let mut input = DocumentInput::new();
    Document::focus(&doc, Some(range));

    // When
    let (down_route, _) = give(&mut input, &doc, &press(" ", "Space", true, false));
    let (up_route, _) = give(&mut input, &doc, &press(" ", "Space", false, false));

    // Then: the slider leaves Space to the host; its up reaches both
    assert_eq!((down_route, up_route), (Route::Both, Route::Both));
    assert_eq!(*heard.borrow(), [" "]);
}

#[test]
fn h_held_then_shift_comes_up_as_capital_h_where_it_went_down() {
    // Given: the slider focused; H down, left to the host
    let (doc, _, range) = card();
    let mut input = DocumentInput::new();
    Document::focus(&doc, Some(range));
    let (down_route, _) = give(&mut input, &doc, &press("h", "KeyH", true, false));

    // When: Shift goes down, then H comes up named "H"
    give(&mut input, &doc, &press("Shift", "ShiftLeft", true, true));
    let (up_route, _) = give(&mut input, &doc, &press("H", "KeyH", false, true));

    // Then
    assert_eq!((down_route, up_route), (Route::Both, Route::Both));
}

#[test]
fn a_keys_up_goes_where_its_down_went_though_a_click_focused_the_card_meanwhile() {
    // Given: H held with nothing focused, so the host has it
    let (doc, button, _) = card();
    let mut input = DocumentInput::new();
    let h = |pressed| Event::Key {
        key: "h".into(),
        code: "KeyH".into(),
        pressed,
        repeat: false,
        modifiers: Modifiers::default(),
    };
    let (down_route, _) = give(&mut input, &doc, &h(true));

    // When: a click focuses the card's button, then H comes up
    click(&mut input, &doc, 20.0, 10.0);
    let (up_route, _) = give(&mut input, &doc, &h(false));

    // Then
    assert_eq!(doc.borrow().active_element(), Some(button));
    assert_eq!((down_route, up_route), (Route::Host, Route::Host));
}

#[test]
fn holding_plus_with_the_slider_focused_the_host_acts_once_a_repeat() {
    // Given
    let (doc, _, range) = card();
    let mut input = DocumentInput::new();
    Document::focus(&doc, Some(range));
    let plus = |pressed, repeat| Event::Key {
        key: "+".into(),
        code: "Equal".into(),
        pressed,
        repeat,
        modifiers: Modifiers {
            shift: true,
            ..Modifiers::default()
        },
    };

    // When: the down, three repeats and the up; the host acts on
    // Host or Both, as Given::route says, and on nothing else
    let mut acted = 0;
    for event in [
        plus(true, false),
        plus(true, true),
        plus(true, true),
        plus(true, true),
        plus(false, false),
    ] {
        let (route, _) = give(&mut input, &doc, &event);
        acted += usize::from(matches!(route, Route::Host | Route::Both));
    }

    // Then: once for each of the five
    assert_eq!(acted, 5);
}

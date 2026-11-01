use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, Event as DomEvent, NodeId};
use vapors_elements::{Cursor, Document, Viewport};
use vapors_platform::{Event, Modifiers, TouchPhase};
use vapors_text::FontSystem;

use super::Route;
use crate::DocumentInput;

/// A 100x40 card at the top left, with a button, over a body that lets
/// the pointer through: the card captures on pointerdown, counts the
/// moves it gets, and shows `move` while `:active`.
struct Card {
    doc: Rc<RefCell<Document>>,
    button: NodeId,
    moves: Rc<RefCell<u32>>,
    clicks: Rc<RefCell<u32>>,
}

fn card() -> Card {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "card.css",
        "html, body { margin: 0; pointer-events: none; } \
         div { position: absolute; left: 0; top: 0; width: 100px; height: 40px; pointer-events: auto; cursor: pointer; display: flex; } \
         div:active { cursor: move; } button { width: 40px; height: 20px; }",
    )
    .expect("parses");
    let body = doc.body();
    let card = doc.create_element("div");
    doc.append(body, card);
    let button = doc.create_element("button");
    doc.append(card, button);
    let (moves, clicks) = (Rc::new(RefCell::new(0)), Rc::new(RefCell::new(0)));
    doc.listen(
        card,
        "pointerdown",
        Box::new(|e: &mut DomEvent| e.set_pointer_capture()),
    );
    let counted = Rc::clone(&moves);
    doc.listen(
        card,
        "pointermove",
        Box::new(move |_: &mut DomEvent| *counted.borrow_mut() += 1),
    );
    let clicked = Rc::clone(&clicks);
    doc.listen(
        card,
        "click",
        Box::new(move |_: &mut DomEvent| *clicked.borrow_mut() += 1),
    );
    doc.update(Viewport {
        width: 200.0,
        height: 100.0,
        scale: 1.0,
    })
    .expect("updates");
    Card {
        doc: Rc::new(RefCell::new(doc)),
        button,
        moves,
        clicks,
    }
}

/// Route `event` as a host would, giving the document what is its.
fn give(input: &mut DocumentInput, card: &Card, event: &Event) -> Route {
    let route = input.route(&card.doc, event).expect("placed");
    if route == Route::Document {
        input.handle(&card.doc, event).expect("placed");
        card.doc
            .borrow_mut()
            .update(Viewport {
                width: 200.0,
                height: 100.0,
                scale: 1.0,
            })
            .expect("updates");
    }
    route
}

fn moved(x: f32, y: f32) -> Event {
    Event::PointerMoved { x, y }
}

fn button(pressed: bool) -> Event {
    Event::PrimaryButton {
        pressed,
        clicks: std::num::NonZeroU32::MIN,
    }
}

#[test]
fn a_press_on_the_card_released_off_it_ends_there_and_holds_nothing() {
    // Given
    let card = card();
    let mut input = DocumentInput::new();
    give(&mut input, &card, &moved(80.0, 30.0));

    // When: press on the card, drag onto the photo, release there
    let pressed = give(&mut input, &card, &button(true));
    give(&mut input, &card, &moved(150.0, 80.0));
    let released = give(&mut input, &card, &button(false));
    let moves_after_release = *card.moves.borrow();
    give(&mut input, &card, &moved(160.0, 80.0));

    // Then: the release reached the card; no capture, drag or :active is left
    assert_eq!((pressed, released), (Route::Document, Route::Document));
    assert_eq!(
        *card.moves.borrow(),
        moves_after_release,
        "a captured move after the release"
    );
    assert_eq!(
        card.doc.borrow().cursor([80.0, 30.0]),
        Cursor::Pointer,
        "still :active"
    );
}

#[test]
fn a_click_on_the_photo_after_moves_over_it_never_lands_on_the_card() {
    // Given: over the card, then over the photo
    let card = card();
    let mut input = DocumentInput::new();
    give(&mut input, &card, &moved(80.0, 30.0));
    give(&mut input, &card, &moved(150.0, 80.0));

    // When
    let pressed = give(&mut input, &card, &button(true));
    let released = give(&mut input, &card, &button(false));

    // Then
    assert_eq!((pressed, released), (Route::Host, Route::Host));
    assert_eq!(*card.clicks.borrow(), 0);
}

#[test]
fn the_wheel_and_a_tap_go_by_hit_at_their_own_position() {
    // Given
    let card = card();
    let mut input = DocumentInput::new();
    let wheel = Event::Scroll { dx: 0.0, dy: 10.0 };

    // When / Then
    give(&mut input, &card, &moved(150.0, 80.0));
    assert_eq!(give(&mut input, &card, &wheel), Route::Host);
    give(&mut input, &card, &moved(80.0, 30.0));
    assert_eq!(give(&mut input, &card, &wheel), Route::Document);
    assert_eq!(
        give(&mut input, &card, &Event::Tap { x: 150.0, y: 80.0 }),
        Route::Host
    );
}

#[test]
fn a_touch_holds_its_route_until_it_ends_and_its_gestures_pans_follow_the_first() {
    // Given
    let card = card();
    let mut input = DocumentInput::new();
    let touch = |id, phase, x, y| Event::Touch { id, phase, x, y };

    // When: a finger down on the card, a pan, the finger up over the photo
    let down = give(
        &mut input,
        &card,
        &touch(1, TouchPhase::Started, 80.0, 30.0),
    );
    let pan = give(&mut input, &card, &Event::Pan { dx: 5.0, dy: 0.0 });
    let up = give(&mut input, &card, &touch(1, TouchPhase::Ended, 150.0, 80.0));
    let on_photo = give(
        &mut input,
        &card,
        &touch(2, TouchPhase::Started, 150.0, 80.0),
    );
    let photo_pan = give(&mut input, &card, &Event::Pan { dx: 5.0, dy: 0.0 });

    // Then
    assert_eq!([down, pan, up], [Route::Document; 3]);
    assert_eq!([on_photo, photo_pan], [Route::Host; 2]);
}

#[test]
fn keys_and_typed_text_follow_the_documents_focus() {
    // Given
    let card = card();
    let mut input = DocumentInput::new();
    let tab = Event::Key {
        key: "Tab".into(),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::default(),
    };
    let typed = Event::Character("a".into());

    // When
    let unfocused = [input.route(&card.doc, &tab), input.route(&card.doc, &typed)];
    Document::focus(&card.doc, Some(card.button));
    let focused = [input.route(&card.doc, &tab), input.route(&card.doc, &typed)];

    // Then
    assert_eq!(unfocused, [Ok(Route::Host); 2]);
    assert_eq!(focused, [Ok(Route::Document); 2]);
}

//! Keys between the card and the host: a press on the host blurs the
//! card, and a key the focused card does not use is the host's.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_platform::{Event, Modifiers, Platform};
use vapors_text::FontSystem;

use super::Route;
use crate::{DocumentInput, Given};

pub(super) const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

/// A card at the top left, 120x40, holding a 40x20 button and an 80x20
/// range 0..10 at 5, over a body that lets the pointer through.
pub(super) fn card() -> (Rc<RefCell<Document>>, NodeId, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "card.css",
        "html, body { margin: 0; pointer-events: none; } \
         div { display: flex; width: 120px; height: 40px; pointer-events: auto; } \
         button { width: 40px; height: 20px; } input { width: 80px; height: 20px; }",
    )
    .expect("parses");
    let body = doc.body();
    let card = doc.create_element("div");
    doc.append(body, card);
    let button = doc.create_element("button");
    doc.append(card, button);
    let range = doc.create_element("input");
    for (name, value) in [
        ("type", "range"),
        ("min", "0"),
        ("max", "10"),
        ("value", "5"),
    ] {
        doc.set_attribute(range, name, value);
    }
    doc.append(card, range);
    doc.update(VIEWPORT).expect("updates");
    (Rc::new(RefCell::new(doc)), button, range)
}

/// `event` given as a host gives it, then the document updated: who it
/// was for in the end, and what it did.
pub(super) fn give(
    input: &mut DocumentInput,
    doc: &Rc<RefCell<Document>>,
    event: &Event,
) -> (Route, Given) {
    let given = input
        .give(doc, event, &Platform::detached())
        .expect("placed");
    doc.borrow_mut().update(VIEWPORT).expect("updates");
    (given.route, given)
}

pub(super) fn click(input: &mut DocumentInput, doc: &Rc<RefCell<Document>>, x: f32, y: f32) {
    give(input, doc, &Event::PointerMoved { x, y });
    give(
        input,
        doc,
        &Event::PrimaryButton {
            pressed: true,
            clicks: std::num::NonZeroU32::MIN,
            modifiers: vapors_platform::Modifiers::default(),
        },
    );
    give(
        input,
        doc,
        &Event::PrimaryButton {
            pressed: false,
            clicks: std::num::NonZeroU32::MIN,
            modifiers: vapors_platform::Modifiers::default(),
        },
    );
}

/// The DOM `code` of a key on a US layout, for the keys these tests use.
fn code_of(name: &str) -> String {
    match name {
        " " => "Space".into(),
        "-" => "Minus".into(),
        n if n.len() == 1 && n.chars().all(|c| c.is_ascii_digit()) => format!("Digit{n}"),
        n if n.len() == 1 => format!("Key{}", n.to_ascii_uppercase()),
        n => n.into(),
    }
}

pub(super) fn key(name: &str, modifiers: Modifiers) -> Event {
    Event::Key {
        key: name.into(),
        code: code_of(name),
        pressed: true,
        repeat: false,
        modifiers,
    }
}

#[test]
fn a_click_on_the_photo_after_a_click_on_the_cards_button_sends_keys_to_the_host() {
    // Given: the card's button focused by a click
    let (doc, button, _) = card();
    let mut input = DocumentInput::new();
    click(&mut input, &doc, 20.0, 10.0);
    assert_eq!(doc.borrow().active_element(), Some(button));

    // When: a click on the photo, then a key
    click(&mut input, &doc, 150.0, 80.0);
    let (route, _) = give(&mut input, &doc, &key("z", Modifiers::default()));

    // Then
    assert_eq!(doc.borrow().active_element(), None);
    assert_eq!(route, Route::Host);
}

#[test]
fn with_the_slider_focused_its_arrow_moves_it_and_an_unbound_key_reaches_the_host() {
    // Given
    let (doc, _, range) = card();
    let mut input = DocumentInput::new();
    Document::focus(&doc, Some(range));

    // When
    let (arrow_route, _) = give(&mut input, &doc, &key("ArrowRight", Modifiers::default()));
    let control = Modifiers {
        control: true,
        ..Modifiers::default()
    };
    let (undo_route, _) = give(&mut input, &doc, &key("z", control));

    // Then
    assert_eq!(arrow_route, Route::Document, "the slider used it");
    assert_eq!(doc.borrow().value(range).as_deref(), Some("6"));
    assert_eq!(undo_route, Route::Both, "Control+Z is the host's too");
}

#[test]
fn typing_minus_one_into_a_focused_number_field_leaves_the_host_no_key() {
    // Given: a number field on the card, focused
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("card.css", "body { display: flex; } input { width: 6em; }")
        .expect("parses");
    let body = doc.body();
    let field = doc.create_element("input");
    doc.set_attribute(field, "type", "number");
    doc.append(body, field);
    doc.update(VIEWPORT).expect("updates");
    let doc = Rc::new(RefCell::new(doc));
    Document::focus(&doc, Some(field));
    let mut input = DocumentInput::new();

    // When: "-" then "1", each a key-down and its text
    let mut host_keys = Vec::new();
    for typed in ["-", "1"] {
        let (route, _) = give(&mut input, &doc, &key(typed, Modifiers::default()));
        give(&mut input, &doc, &Event::Character(typed.into()));
        if matches!(route, Route::Host | Route::Both) {
            host_keys.push(typed);
        }
    }

    // Then
    assert_eq!(host_keys, Vec::<&str>::new());
    assert_eq!(doc.borrow().value(field).as_deref(), Some("-1"));
}

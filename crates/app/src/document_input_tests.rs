use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, Event as DomEvent};
use vapors_elements::{Document, Viewport};
use vapors_platform::{Cursor as Shown, Event};
use vapors_text::FontSystem;

use super::{DocumentInput, Unplaced};

/// A 100x20 CSS-pixel button at the top left with a pointer cursor,
/// updated at `scale`, and a count of its clicks.
fn button_at(scale: f32) -> (Rc<RefCell<Document>>, Rc<RefCell<u32>>) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; display: flex; } button { width: 100px; height: 20px; cursor: pointer; }",
    )
    .expect("parses");
    let body = doc.body();
    let button = doc.create_element("button");
    doc.append(body, button);
    let clicks = Rc::new(RefCell::new(0));
    let counter = Rc::clone(&clicks);
    doc.listen(
        button,
        "click",
        Box::new(move |_: &mut DomEvent| *counter.borrow_mut() += 1),
    );
    doc.update(Viewport {
        width: 200.0,
        height: 100.0,
        scale,
    })
    .expect("updates");
    doc.take_changed();
    (Rc::new(RefCell::new(doc)), clicks)
}

fn moved(x: f32, y: f32) -> Event {
    Event::PointerMoved { x, y }
}

#[test]
fn an_event_lands_at_the_scale_of_the_documents_last_update() {
    // Given
    let (document, _) = button_at(2.0);
    let mut input = DocumentInput::new();

    // When
    let at = input.at(&document.borrow(), &moved(40.0, 20.0));
    input.handle(&document, &moved(40.0, 20.0)).expect("placed");

    // Then: a button lands where the pointer last moved
    assert_eq!(at, Ok(Some([20.0, 10.0])));
    let press = Event::PrimaryButton {
        pressed: true,
        clicks: std::num::NonZeroU32::MIN,
    };
    assert_eq!(input.at(&document.borrow(), &press), Ok(Some([20.0, 10.0])));
    assert_eq!(input.at(&document.borrow(), &Event::PointerLeft), Ok(None));
}

#[test]
fn a_press_and_release_on_the_button_clicks_it() {
    // Given
    let (document, clicks) = button_at(2.0);
    let mut input = DocumentInput::new();
    input.handle(&document, &moved(40.0, 20.0)).expect("placed");

    // When
    input
        .handle(
            &document,
            &Event::PrimaryButton {
                pressed: true,
                clicks: std::num::NonZeroU32::MIN,
            },
        )
        .expect("placed");
    input
        .handle(
            &document,
            &Event::PrimaryButton {
                pressed: false,
                clicks: std::num::NonZeroU32::MIN,
            },
        )
        .expect("placed");

    // Then
    assert_eq!(*clicks.borrow(), 1);
}

#[test]
fn the_cursor_is_reported_once_on_a_change_and_not_on_each_move() {
    // Given
    let (document, _) = button_at(2.0);
    let mut input = DocumentInput::new();

    // When
    let onto = input.handle(&document, &moved(40.0, 20.0)).expect("placed");
    let along = input.handle(&document, &moved(60.0, 20.0)).expect("placed");
    let off = input
        .handle(&document, &moved(300.0, 150.0))
        .expect("placed");

    // Then
    assert_eq!(onto.cursor, Some(Shown::Pointer));
    assert_eq!(along.cursor, None);
    assert_eq!(off.cursor, Some(Shown::Default));
}

#[test]
fn after_the_host_shows_its_own_cursor_the_documents_is_reported_again() {
    // Given: off the button (default), the host shows a crosshair
    let (document, _) = button_at(2.0);
    let mut input = DocumentInput::new();
    input
        .handle(&document, &moved(300.0, 150.0))
        .expect("placed");
    input.forget_cursor();

    // When: still over nothing of the document's
    let back = input
        .handle(&document, &moved(310.0, 150.0))
        .expect("placed");

    // Then
    assert_eq!(back.cursor, Some(Shown::Default));
}

#[test]
fn input_before_the_first_update_or_at_a_zero_scale_is_refused() {
    // Given
    let never = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let (zero, _) = button_at(0.0);
    let mut input = DocumentInput::new();

    // Then
    assert_eq!(
        input.handle(&never, &moved(1.0, 1.0)),
        Err(Unplaced::NotUpdated)
    );
    assert_eq!(
        input.handle(&zero, &moved(1.0, 1.0)),
        Err(Unplaced::Scale(0.0))
    );
}

#[test]
fn a_key_reaches_a_document_never_updated_and_a_pointer_event_is_still_refused() {
    // Given: a focused button in a document never updated
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    let button = doc.create_element("button");
    doc.append(body, button);
    let keys = Rc::new(RefCell::new(0));
    let counted = Rc::clone(&keys);
    doc.listen(
        body,
        "keydown",
        Box::new(move |_: &mut DomEvent| *counted.borrow_mut() += 1),
    );
    let document = Rc::new(RefCell::new(doc));
    Document::focus(&document, Some(button));
    let mut input = DocumentInput::new();
    let key = Event::Key {
        key: "z".into(),
        pressed: true,
        repeat: false,
        modifiers: vapors_platform::Modifiers::default(),
    };

    // When
    let routed = input.route(&document, &key);
    let given = input.handle(&document, &key);
    let pointer = input.handle(&document, &moved(1.0, 1.0));

    // Then
    assert_eq!(routed, Ok(super::Route::Document));
    assert!(given.is_ok(), "{given:?}");
    assert_eq!(*keys.borrow(), 1);
    assert_eq!(pointer, Err(Unplaced::NotUpdated));
}

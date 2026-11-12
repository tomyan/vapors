//! A paste through `DocumentInput`, as an overlay host routes it: with a
//! card text field focused, Control+V and the paste are the card's, and
//! the field gets the clipboard's text; with none focused the paste is
//! the host's and the clipboard is not read.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, ShortcutModifier, Viewport};
use vapors_platform::{
    Clipboard, ClipboardDone, ClipboardImage, ClipboardText, Event, Modifiers, Platform,
};
use vapors_text::FontSystem;

use crate::{DocumentInput, Route};

/// A clipboard holding "Glow", counting its reads.
struct Glow(Rc<RefCell<u32>>);

impl Clipboard for Glow {
    fn image(&mut self) -> ClipboardImage {
        ClipboardImage::NoImage
    }

    fn read_text(&mut self, done: ClipboardDone<ClipboardText>) {
        *self.0.borrow_mut() += 1;
        done(ClipboardText::Text("Glow".into()));
    }
}

/// A card with an empty 160 px text field.
fn card() -> (Rc<RefCell<Document>>, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "card.css",
        "body { margin: 0; } input { display: block; width: 160px; }",
    )
    .expect("parses");
    let body = doc.body();
    let field = doc.create_element("input");
    doc.append(body, field);
    doc.update(Viewport {
        width: 200.0,
        height: 100.0,
        scale: 1.0,
    })
    .expect("updates");
    (Rc::new(RefCell::new(doc)), field)
}

fn control_v() -> Event {
    v_with(Modifiers {
        control: true,
        ..Modifiers::default()
    })
}

fn command_v() -> Event {
    v_with(Modifiers {
        meta: true,
        ..Modifiers::default()
    })
}

fn v_with(modifiers: Modifiers) -> Event {
    Event::Key {
        key: "v".into(),
        code: "KeyV".into(),
        pressed: true,
        repeat: false,
        modifiers,
    }
}

#[test]
fn control_v_in_a_focused_card_field_pastes_there_and_leaves_the_host_nothing() {
    // Given
    let (doc, field) = card();
    Document::focus(&doc, Some(field));
    let reads = Rc::new(RefCell::new(0));
    let mut input = DocumentInput::new().with_clipboard(Box::new(Glow(Rc::clone(&reads))));
    let platform = Platform::detached();

    // When: the host gives Control+V, then the platform's paste
    let key = input.give(&doc, &control_v(), &platform).expect("placed");
    let paste = input.give(&doc, &Event::Paste, &platform).expect("placed");
    input.deliver_pastes(&doc);

    // Then
    assert_eq!(
        key.route,
        Route::Document,
        "the field's paste, not the host's"
    );
    assert_eq!(
        paste.route,
        Route::Document,
        "the host's image paste would run"
    );
    assert_eq!(doc.borrow().value(field).as_deref(), Some("Glow"));
    assert_eq!(*reads.borrow(), 1);
}

#[test]
fn with_no_card_field_focused_the_paste_is_the_hosts_and_nothing_is_read() {
    // Given
    let (doc, _) = card();
    let reads = Rc::new(RefCell::new(0));
    let mut input = DocumentInput::new().with_clipboard(Box::new(Glow(Rc::clone(&reads))));

    // When
    let paste = input
        .give(&doc, &Event::Paste, &Platform::detached())
        .expect("placed");

    // Then
    assert_eq!(paste.route, Route::Host);
    assert_eq!(*reads.borrow(), 0);
}

/// The route of `key` going down in a focused card field, on a platform
/// whose shortcuts take `modifier`.
fn paste_key_route(modifier: ShortcutModifier, key: &Event) -> Route {
    let (doc, field) = card();
    doc.borrow_mut().set_shortcut_modifier(modifier);
    Document::focus(&doc, Some(field));
    let mut input = DocumentInput::new();
    input
        .give(&doc, key, &Platform::detached())
        .expect("placed")
        .route
}

#[test]
fn command_v_is_the_fields_paste_on_macos_and_control_v_is_not() {
    assert_eq!(
        paste_key_route(ShortcutModifier::Command, &command_v()),
        Route::Document
    );
    assert_ne!(
        paste_key_route(ShortcutModifier::Command, &control_v()),
        Route::Document
    );
}

#[test]
fn control_v_is_the_fields_paste_elsewhere_and_command_v_is_not() {
    assert_eq!(
        paste_key_route(ShortcutModifier::Control, &control_v()),
        Route::Document
    );
    assert_ne!(
        paste_key_route(ShortcutModifier::Control, &command_v()),
        Route::Document
    );
}

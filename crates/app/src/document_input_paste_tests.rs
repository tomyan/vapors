//! A paste through `DocumentInput`, as an overlay host routes it: with a
//! card text field focused, Control+V and the paste are the card's, and
//! the field gets the clipboard's text; with none focused the paste is
//! the host's and the clipboard is not read.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
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
    let modifiers = Modifiers {
        control: true,
        ..Modifiers::default()
    };
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
    let mut input = DocumentInput::new();
    let reads = Rc::new(RefCell::new(0));
    let mut clipboard = Glow(Rc::clone(&reads));
    let platform = Platform::detached();

    // When: the host routes Control+V, then the platform's paste
    let key_route = input.route(&doc, &control_v()).expect("placed");
    let key = input.handle(&doc, &control_v()).expect("placed");
    let paste_route = input.route(&doc, &Event::Paste).expect("placed");
    let taken = input.paste(&doc, &mut clipboard, &platform);
    input.deliver_pastes(&doc);

    // Then
    assert_eq!((key_route, key.unused_key), (Route::Document, false));
    assert_eq!(
        paste_route,
        Route::Document,
        "the host's image paste would run"
    );
    assert!(taken);
    assert_eq!(doc.borrow().value(field).as_deref(), Some("Glow"));
    assert_eq!(*reads.borrow(), 1);
}

#[test]
fn with_no_card_field_focused_the_paste_is_the_hosts_and_nothing_is_read() {
    // Given
    let (doc, _) = card();
    let mut input = DocumentInput::new();
    let reads = Rc::new(RefCell::new(0));
    let mut clipboard = Glow(Rc::clone(&reads));

    // When
    let route = input.route(&doc, &Event::Paste).expect("placed");
    let taken = input.paste(&doc, &mut clipboard, &Platform::detached());

    // Then
    assert_eq!(route, Route::Host);
    assert!(!taken);
    assert_eq!(*reads.borrow(), 0);
}

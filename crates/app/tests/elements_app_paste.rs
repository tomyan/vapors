//! A paste (`Event::Paste`, Control+V on the desktop) through
//! `ElementsApp::event`: a focused text field takes the clipboard's
//! text first; with none focused the clipboard is not read; a clipboard
//! with no text pastes nothing. Native only: the web page has its own
//! paste.
#![cfg(not(target_family = "wasm"))]

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_platform::{App, Clipboard, ClipboardImage, ClipboardText, Event, Platform};
use vapors_text::FontSystem;

/// A clipboard holding `text`, counting its reads.
struct Held {
    text: ClipboardText,
    reads: Rc<RefCell<u32>>,
}

impl Clipboard for Held {
    fn image(&mut self) -> ClipboardImage {
        ClipboardImage::NoImage
    }

    fn text(&mut self) -> ClipboardText {
        *self.reads.borrow_mut() += 1;
        self.text.clone()
    }
}

/// A 160 px text field holding "Warm" at the top left.
fn field() -> (Rc<RefCell<Document>>, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } input { display: block; width: 160px; }",
    )
    .expect("parses");
    let body = doc.body();
    let input = doc.create_element("input");
    doc.set_attribute(input, "value", "Warm");
    doc.append(body, input);
    doc.update(Viewport {
        width: 200.0,
        height: 100.0,
        scale: 1.0,
    })
    .expect("updates");
    (Rc::new(RefCell::new(doc)), input)
}

fn app(doc: &Rc<RefCell<Document>>, text: ClipboardText) -> (ElementsApp, Rc<RefCell<u32>>) {
    let reads = Rc::new(RefCell::new(0));
    let clipboard = Held {
        text,
        reads: Rc::clone(&reads),
    };
    let app = ElementsApp::new(Rc::clone(doc)).with_clipboard(Box::new(clipboard));
    (app, reads)
}

/// A click at the field's right, putting the caret after "Warm".
fn click_field(app: &mut ElementsApp, platform: &Platform) {
    for event in [
        Event::PointerMoved { x: 150.0, y: 10.0 },
        Event::PrimaryButton {
            pressed: true,
            clicks: std::num::NonZeroU32::MIN,
        },
        Event::PrimaryButton {
            pressed: false,
            clicks: std::num::NonZeroU32::MIN,
        },
    ] {
        app.event(event, platform);
    }
}

#[test]
fn a_paste_goes_to_the_focused_text_field_first() {
    // Given
    let (doc, input) = field();
    let (mut app, _) = app(&doc, ClipboardText::Text("th".into()));
    let platform = Platform::detached();
    click_field(&mut app, &platform);

    // When
    app.event(Event::Paste, &platform);

    // Then
    assert_eq!(doc.borrow().value(input).as_deref(), Some("Warmth"));
    assert!(platform.take_redraw_request());
}

#[test]
fn with_no_text_field_focused_the_clipboard_is_not_read() {
    // Given: nothing focused
    let (doc, input) = field();
    let (mut app, reads) = app(&doc, ClipboardText::Text("th".into()));
    let platform = Platform::detached();

    // When
    app.event(Event::Paste, &platform);

    // Then
    assert_eq!(*reads.borrow(), 0);
    assert_eq!(doc.borrow().value(input).as_deref(), Some("Warm"));
}

#[test]
fn a_clipboard_with_no_text_pastes_nothing() {
    // Given
    let (doc, input) = field();
    let (mut app, reads) = app(&doc, ClipboardText::NoText);
    let platform = Platform::detached();
    click_field(&mut app, &platform);

    // When
    app.event(Event::Paste, &platform);

    // Then
    assert_eq!(*reads.borrow(), 1);
    assert_eq!(doc.borrow().value(input).as_deref(), Some("Warm"));
}

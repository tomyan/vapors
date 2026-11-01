//! A paste (`Event::Paste`, Control+V on the desktop) through
//! `ElementsApp::event`: a focused text field takes the clipboard's
//! text first, read off the event loop and pasted once it arrives; with
//! none focused the clipboard is not read; a clipboard with no text
//! pastes nothing. Native only: the web page has its own paste.
#![cfg(not(target_family = "wasm"))]

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_elements::Document;
use vapors_platform::{
    App, Clipboard, ClipboardDone, ClipboardImage, ClipboardText, Event, Platform,
};

#[path = "support/paste_page.rs"]
mod paste_page;

use paste_page::{click_field, field};

/// A clipboard holding `text`, counting its reads.
struct Held {
    text: ClipboardText,
    reads: Rc<RefCell<u32>>,
}

impl Clipboard for Held {
    fn image(&mut self) -> ClipboardImage {
        ClipboardImage::NoImage
    }

    fn read_text(&mut self, done: ClipboardDone<ClipboardText>) {
        *self.reads.borrow_mut() += 1;
        done(self.text.clone());
    }
}

/// A clipboard that answers `text` from another thread after 200 ms.
struct Slow(&'static str);

impl Clipboard for Slow {
    fn image(&mut self) -> ClipboardImage {
        ClipboardImage::NoImage
    }

    fn read_text(&mut self, done: ClipboardDone<ClipboardText>) {
        let text = self.0;
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(200));
            done(ClipboardText::Text(text.to_owned()));
        });
    }
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

#[test]
fn a_paste_goes_to_the_focused_text_field_first() {
    // Given
    let (doc, input) = field();
    let (mut app, _) = app(&doc, ClipboardText::Text("th".into()));
    let platform = Platform::detached();
    click_field(&mut app, &platform);

    // When
    app.event(Event::Paste, &platform);
    app.deliver_pastes();

    // Then
    assert_eq!(doc.borrow().value(input).as_deref(), Some("Warmth"));
    assert!(platform.take_redraw_request());
}

#[test]
fn a_paste_does_not_wait_for_the_clipboard_and_lands_when_it_answers() {
    // Given
    let (doc, input) = field();
    let mut app = ElementsApp::new(Rc::clone(&doc)).with_clipboard(Box::new(Slow("th")));
    let platform = Platform::detached();
    click_field(&mut app, &platform);
    let _ = platform.take_redraw_request();

    // When: the event returns before the clipboard answers
    app.event(Event::Paste, &platform);
    app.deliver_pastes();
    let before = doc.borrow().value(input);

    // Then: the answer wakes the loop (5 s, 25 times the clipboard's 200 ms)
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !platform.take_redraw_request() {
        assert!(
            std::time::Instant::now() < deadline,
            "the clipboard's answer woke nothing"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    app.deliver_pastes();
    assert_eq!(before.as_deref(), Some("Warm"));
    assert_eq!(doc.borrow().value(input).as_deref(), Some("Warmth"));
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
    app.deliver_pastes();

    // Then
    assert_eq!(*reads.borrow(), 1);
    assert_eq!(doc.borrow().value(input).as_deref(), Some("Warm"));
}

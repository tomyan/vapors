//! Platform events through `ElementsApp::event`, with no window: input
//! in physical pixels reaches the document at the display scale, a
//! press and release on a button clicks it, and a frame is asked for
//! exactly when the document changed.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_dom::{Dom, Event as DomEvent};
use vapors_elements::{Document, Viewport};
use vapors_platform::{App, Event, Platform};
use vapors_text::FontSystem;

/// A 100x20 CSS-pixel button at the top left, updated at 2x, and a
/// count of its clicks.
fn setup() -> (Rc<RefCell<Document>>, Rc<RefCell<u32>>) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; display: flex; } button { width: 100px; height: 20px; }",
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
        scale: 2.0,
    })
    .expect("updates");
    doc.take_changed();
    (Rc::new(RefCell::new(doc)), clicks)
}

#[test]
fn a_press_and_release_in_physical_pixels_click_the_button_at_2x() {
    // Given: the button covers CSS x 0..100, so physical x 150 is CSS 75
    let (doc, clicks) = setup();
    let mut app = ElementsApp::new(Rc::clone(&doc)).with_scale(2.0);
    let platform = Platform::detached();

    // When
    for event in [
        Event::PointerMoved { x: 150.0, y: 10.0 },
        Event::PrimaryButton { pressed: true },
        Event::PrimaryButton { pressed: false },
    ] {
        app.event(event, &platform);
    }

    // Then
    assert_eq!(*clicks.borrow(), 1);
    assert!(
        platform.take_redraw_request(),
        ":hover and :active changed, so a frame is asked for"
    );
}

#[test]
fn a_press_outside_the_button_at_2x_clicks_nothing() {
    // Given: physical x 250 is CSS 125, past the button's 100
    let (doc, clicks) = setup();
    let mut app = ElementsApp::new(Rc::clone(&doc)).with_scale(2.0);
    let platform = Platform::detached();

    // When
    for event in [
        Event::PointerMoved { x: 250.0, y: 10.0 },
        Event::PrimaryButton { pressed: true },
        Event::PrimaryButton { pressed: false },
    ] {
        app.event(event, &platform);
    }

    // Then
    assert_eq!(*clicks.borrow(), 0);
}

#[test]
fn an_event_that_changes_nothing_asks_for_no_frame() {
    // Given
    let (doc, _) = setup();
    let mut app = ElementsApp::new(Rc::clone(&doc)).with_scale(2.0);
    let platform = Platform::detached();

    // When: a key the document does not take
    app.event(Event::Character("x".into()), &platform);

    // Then
    assert!(!platform.take_redraw_request());
}

#[test]
fn a_tab_key_moves_focus_to_the_button_and_enter_clicks_it() {
    // Given
    let (doc, clicks) = setup();
    let mut app = ElementsApp::new(Rc::clone(&doc)).with_scale(2.0);
    let platform = Platform::detached();
    let key = |key: &str, pressed| Event::Key {
        key: key.to_owned(),
        pressed,
        repeat: false,
        modifiers: vapors_platform::Modifiers::default(),
    };

    // When
    for event in [
        key("Tab", true),
        key("Tab", false),
        key("Enter", true),
        key("Enter", false),
    ] {
        app.event(event, &platform);
    }

    // Then
    assert!(doc.borrow().active_element().is_some());
    assert_eq!(*clicks.borrow(), 1);
}

//! Platform events through `ElementsApp::give`, for an app that wraps
//! the `ElementsApp` and draws under its document: each event answers
//! who it was for (`Route`), the document gets only what is its, and an
//! event that cannot be placed is refused as `Unplaced`.

use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;

use vapors_app::{ElementsApp, Route, Unplaced};
use vapors_dom::{Dom, Event as DomEvent};
use vapors_elements::{Document, Viewport};
use vapors_platform::{Event, Modifiers, Platform};
use vapors_text::FontSystem;

/// A 100x20 button at the top left of a 200x100 page whose body takes
/// no pointer (the host's drawing shows through it), and a count of its
/// clicks; updated at 1x when `updated`.
fn page(updated: bool) -> (ElementsApp, Rc<RefCell<Document>>, Rc<RefCell<u32>>) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "html, body { pointer-events: none; } body { margin: 0; display: flex; } \
        button { pointer-events: auto; width: 100px; height: 20px; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    let (body, button) = (doc.body(), doc.create_element("button"));
    doc.append(body, button);
    let clicks = Rc::new(RefCell::new(0));
    let counter = Rc::clone(&clicks);
    let click = move |_: &mut DomEvent| *counter.borrow_mut() += 1;
    doc.listen(button, "click", Box::new(click));
    if updated {
        let viewport = Viewport {
            width: 200.0,
            height: 100.0,
            scale: 1.0,
        };
        doc.update(viewport).expect("updates");
    }
    let doc = Rc::new(RefCell::new(doc));
    (ElementsApp::new(Rc::clone(&doc)), doc, clicks)
}

/// A press and release at `x`: the press's route and the release's.
fn click_at(app: &mut ElementsApp, platform: &Platform, x: f32) -> Vec<Result<Route, Unplaced>> {
    let button = |pressed| Event::PrimaryButton {
        pressed,
        clicks: NonZeroU32::MIN,
        modifiers: Modifiers::default(),
    };
    app.give(&Event::PointerMoved { x, y: 10.0 }, platform)
        .expect("a move is placed");
    [button(true), button(false)]
        .iter()
        .map(|event| app.give(event, platform))
        .collect()
}

fn key(code: &str, pressed: bool) -> Event {
    Event::Key {
        key: code.to_owned(),
        code: code.to_owned(),
        pressed,
        repeat: false,
        modifiers: Modifiers::default(),
    }
}

fn focus_button(doc: &Rc<RefCell<Document>>) {
    let elements = doc.borrow().elements();
    let button = elements
        .into_iter()
        .find(|n| doc.borrow().tag(*n) == Some("button"));
    Document::focus(doc, button);
}

#[test]
fn a_press_on_the_button_is_the_documents_and_clicks_it() {
    // Given
    let (mut app, _doc, clicks) = page(true);
    let platform = Platform::detached();

    // When
    let routes = click_at(&mut app, &platform, 50.0);

    // Then
    assert_eq!(routes, [Ok(Route::Document), Ok(Route::Document)]);
    assert_eq!(*clicks.borrow(), 1);
    assert!(platform.take_redraw_request(), "the button's state changed");
}

#[test]
fn a_press_outside_the_document_is_left_for_the_app() {
    // Given: x 150 is past the button, over the body
    let (mut app, _doc, clicks) = page(true);
    let platform = Platform::detached();

    // When
    let routes = click_at(&mut app, &platform, 150.0);

    // Then
    assert_eq!(routes, [Ok(Route::Host), Ok(Route::Host)]);
    assert_eq!(*clicks.borrow(), 0);
}

#[test]
fn a_key_the_focused_button_uses_is_the_documents_and_one_it_leaves_is_both() {
    // Given
    let (mut app, doc, clicks) = page(true);
    let platform = Platform::detached();
    focus_button(&doc);

    // When
    let enter = [key("Enter", true), key("Enter", false)].map(|e| app.give(&e, &platform));
    let other = [key("KeyR", true), key("KeyR", false)].map(|e| app.give(&e, &platform));

    // Then
    assert_eq!(enter, [Ok(Route::Document), Ok(Route::Document)]);
    assert_eq!(*clicks.borrow(), 1);
    assert_eq!(other, [Ok(Route::Both), Ok(Route::Both)]);
}

#[test]
fn a_key_with_nothing_focused_is_the_apps_alone_and_the_document_does_not_get_it() {
    // Given: through `App::event` this Tab would focus the button
    let (mut app, doc, _clicks) = page(true);
    let platform = Platform::detached();

    // When
    let tab = [key("Tab", true), key("Tab", false)].map(|e| app.give(&e, &platform));

    // Then
    assert_eq!(tab, [Ok(Route::Host), Ok(Route::Host)]);
    assert_eq!(doc.borrow().active_element(), None);
}

#[test]
fn a_press_before_the_first_update_is_refused_as_not_updated() {
    // Given
    let (mut app, _doc, clicks) = page(false);
    let platform = Platform::detached();

    // When
    let press = Event::PrimaryButton {
        pressed: true,
        clicks: NonZeroU32::MIN,
        modifiers: Modifiers::default(),
    };
    let given = app.give(&press, &platform);

    // Then
    assert_eq!(given, Err(Unplaced::NotUpdated));
    assert_eq!(*clicks.borrow(), 0);
}

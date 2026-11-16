//! A link followed through `ElementsApp::event`, by click, Enter or tap:
//! its `href` goes to the platform where the modifiers say
//! (`Platform::navigate`, `Platform::open_new`); a detached platform
//! records it. Alt (a download) is refused before the platform.

use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};
use vapors_platform::{App, Event, Followed, Modifiers, NewContext, Platform};
use vapors_text::FontSystem;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

/// A 20 px link to `?demo=counter` at the top left, updated.
fn linked() -> Rc<RefCell<Document>> {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } a { display: block; height: 20px; }",
    )
    .expect("parses");
    let body = doc.body();
    let link = doc.create_element("a");
    doc.set_attribute(link, "href", "?demo=counter");
    let label = doc.create_text("Counter");
    doc.append(link, label);
    doc.append(body, link);
    doc.update(VIEWPORT).expect("updates");
    Rc::new(RefCell::new(doc))
}

fn button(pressed: bool, modifiers: Modifiers) -> Event {
    Event::PrimaryButton {
        pressed,
        clicks: NonZeroU32::MIN,
        modifiers,
    }
}

fn key(name: &str, pressed: bool) -> Event {
    Event::Key {
        key: name.to_owned(),
        code: String::new(),
        pressed,
        repeat: false,
        modifiers: Modifiers::default(),
    }
}

/// What the platform was asked to follow after a click on the link with
/// `modifiers` held.
fn clicked_with(modifiers: Modifiers) -> Vec<Followed> {
    let mut app = ElementsApp::new(linked());
    let platform = Platform::detached();
    app.event(Event::PointerMoved { x: 5.0, y: 5.0 }, &platform);
    app.event(button(true, modifiers), &platform);
    app.event(button(false, modifiers), &platform);
    platform.take_followed()
}

fn followed(new: Option<NewContext>) -> Vec<Followed> {
    vec![("?demo=counter".to_owned(), new)]
}

#[test]
fn a_click_on_a_link_asks_the_platform_to_go_to_its_href() {
    assert_eq!(clicked_with(Modifiers::default()), followed(None));
}

#[test]
fn control_shift_and_alt_clicks_open_a_tab_a_window_and_nothing() {
    let held = |control, shift, alt| Modifiers {
        control,
        shift,
        alt,
        meta: false,
    };
    assert_eq!(
        clicked_with(held(true, false, false)),
        followed(Some(NewContext::Tab))
    );
    assert_eq!(
        clicked_with(held(false, true, false)),
        followed(Some(NewContext::Window))
    );
    // a download is refused before the platform: vapors does not download
    assert_eq!(clicked_with(held(false, false, true)), []);
}

#[test]
fn enter_on_the_focused_link_asks_the_platform_to_go_to_its_href() {
    // Given: the link focused by Tab
    let mut app = ElementsApp::new(linked());
    let platform = Platform::detached();
    app.event(key("Tab", true), &platform);
    app.event(key("Tab", false), &platform);

    // When
    app.event(key("Enter", true), &platform);
    app.event(key("Enter", false), &platform);

    // Then
    assert_eq!(platform.take_followed(), followed(None));
}

#[test]
fn a_tap_on_a_link_asks_the_platform_to_go_to_its_href() {
    // Given
    let mut app = ElementsApp::new(linked());
    let platform = Platform::detached();

    // When
    let tap = Event::Tap {
        x: 5.0,
        y: 5.0,
        clicks: NonZeroU32::MIN,
    };
    app.event(tap, &platform);

    // Then
    assert_eq!(platform.take_followed(), followed(None));
}

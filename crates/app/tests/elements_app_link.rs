//! A link followed through `ElementsApp::event`: the click's `href` goes
//! to the platform (`Platform::navigate`), which on the web sets the
//! page's address; a detached platform records it.

use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};
use vapors_platform::{App, Event, Modifiers, Platform};
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

fn button(pressed: bool) -> Event {
    Event::PrimaryButton {
        pressed,
        clicks: NonZeroU32::MIN,
        modifiers: Modifiers::default(),
    }
}

#[test]
fn a_click_on_a_link_asks_the_platform_to_go_to_its_href() {
    // Given
    let mut app = ElementsApp::new(linked());
    let platform = Platform::detached();

    // When
    app.event(Event::PointerMoved { x: 5.0, y: 5.0 }, &platform);
    app.event(button(true), &platform);
    app.event(button(false), &platform);

    // Then
    assert_eq!(platform.take_navigations(), ["?demo=counter"]);
}

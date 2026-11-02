//! A page with one text field, and clicks on it, for the paste tests.

#![allow(dead_code)] // each test binary uses a subset

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_platform::{App, Event, Platform};
use vapors_text::FontSystem;

/// A 160 px text field holding "Warm" at the top left.
pub fn field() -> (Rc<RefCell<Document>>, NodeId) {
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

/// A click at the field's right, putting the caret after "Warm".
pub fn click_field(app: &mut ElementsApp, platform: &Platform) {
    click_at(app, platform, [150.0, 10.0]);
}

/// A press and release at `at`.
pub fn click_at(app: &mut ElementsApp, platform: &Platform, [x, y]: [f32; 2]) {
    for event in [
        Event::PointerMoved { x, y },
        Event::PrimaryButton {
            pressed: true,
            clicks: std::num::NonZeroU32::MIN,
            modifiers: vapors_platform::Modifiers::default(),
        },
        Event::PrimaryButton {
            pressed: false,
            clicks: std::num::NonZeroU32::MIN,
            modifiers: vapors_platform::Modifiers::default(),
        },
    ] {
        app.event(event, platform);
    }
}

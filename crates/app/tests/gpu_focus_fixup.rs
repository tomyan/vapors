//! GPU tier: the focus fixup in an `ElementsApp` frame. A focused text
//! field that a script hides loses focus in the frame whose update
//! leaves it without a box: `blur` fires, the platform's text input is
//! cleared after the frame, and the frame asks for another, which draws
//! the field's ancestors without `:focus-within`.

#[path = "support/surface.rs"]
mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{app_frame, document};
use vapors_app::ElementsApp;
use vapors_dom::{Dom, Event, NodeId};
use vapors_gpu_harness::gpu;
use vapors_platform::{App, Modifiers, Next, Platform};

/// An app on a 30 x 16 px text field at the top left, and the field.
fn field_app() -> (ElementsApp, Rc<RefCell<vapors_elements::Document>>, NodeId) {
    let css = "input { display: block; width: 30px; height: 16px; } \
               body:focus-within { background-color: #ff0000; }";
    let doc = Rc::new(RefCell::new(document(css, &[(None, "input", "name")])));
    let field = doc.borrow().elements()[2];
    let app = ElementsApp::new(Rc::clone(&doc));
    (app, doc, field)
}

/// Whether the last update drew `:focus-within`'s red.
fn red_drawn(doc: &RefCell<vapors_elements::Document>) -> bool {
    let doc = doc.borrow();
    let items = doc.painted().expect("painted").scene().items();
    let red = vapors_scene::Color::from_rgba8(255, 0, 0, 255);
    items
        .iter()
        .any(|item| matches!(item, vapors_scene::Item::Quad(quad) if quad.background == red))
}

/// A click at `(5, 5)`, in the field.
fn click(app: &mut ElementsApp, platform: &Platform) {
    app.event(
        vapors_platform::Event::PointerMoved { x: 5.0, y: 5.0 },
        platform,
    );
    for pressed in [true, false] {
        let clicks = std::num::NonZeroU32::MIN;
        let modifiers = Modifiers::default();
        let event = vapors_platform::Event::PrimaryButton {
            pressed,
            clicks,
            modifiers,
        };
        app.event(event, platform);
    }
}

#[test]
fn gpu_a_focused_field_hidden_by_a_script_loses_focus_in_that_frame() {
    // Given: the field focused by a click and mirrored, a blur logged
    let (mut app, doc, field) = field_app();
    let (gpu, platform) = (gpu(), Platform::detached());
    app_frame(&mut app, &gpu);
    click(&mut app, &platform);
    app_frame(&mut app, &gpu);
    app.drawn(&platform);
    assert_eq!(
        doc.borrow().active_element(),
        Some(field),
        "the click focused it"
    );
    assert!(red_drawn(&doc), "body:focus-within is drawn");
    drop(platform.take_text_field());
    let blurs = Rc::new(RefCell::new(0));
    let count = Rc::clone(&blurs);
    let handler = Box::new(move |_: &mut Event| *count.borrow_mut() += 1);
    doc.borrow_mut().listen(field, "blur", handler);

    // When
    doc.borrow_mut()
        .set_attribute(field, "style", "display: none");
    let (next, _) = app_frame(&mut app, &gpu);
    app.drawn(&platform);

    // Then
    assert_eq!(doc.borrow().active_element(), None);
    assert_eq!(*blurs.borrow(), 1);
    assert_eq!(platform.take_text_field(), Some(None));
    assert_eq!(next, Next::Continue);
    app_frame(&mut app, &gpu);
    assert!(
        !red_drawn(&doc),
        "the frame asked for drew no :focus-within"
    );
}

//! `ElementsApp` passes a platform key's `code`, `repeat` and modifiers
//! through to the focused element's `keydown` and `keyup`
//! (`document_input_keys.rs`, native and web alike).

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_dom::{Dom, Event as DomEvent, Modifiers as DomModifiers};
use vapors_elements::{Document, Viewport};
use vapors_platform::{Event, Modifiers, Platform};
use vapors_text::FontSystem;

/// Each key event the focused button saw: name, key, code, repeat and
/// modifiers.
type Seen = Rc<RefCell<Vec<(String, String, String, bool, DomModifiers)>>>;

fn page() -> (ElementsApp, Seen) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "button { display: block; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    let (body, button) = (doc.body(), doc.create_element("button"));
    doc.append(body, button);
    let seen = Seen::default();
    for name in ["keydown", "keyup"] {
        let into = Rc::clone(&seen);
        let record = move |e: &mut DomEvent| {
            let key = e.key().to_owned();
            let entry = (
                e.name().to_owned(),
                key,
                e.code().to_owned(),
                e.repeat(),
                e.modifiers(),
            );
            into.borrow_mut().push(entry);
        };
        doc.listen(button, name, Box::new(record));
    }
    let viewport = Viewport {
        width: 200.0,
        height: 100.0,
        scale: 1.0,
    };
    doc.update(viewport).expect("updates");
    let doc = Rc::new(RefCell::new(doc));
    Document::focus(&doc, Some(button));
    (ElementsApp::new(doc), seen)
}

fn key(pressed: bool, modifiers: Modifiers) -> Event {
    Event::Key {
        key: "a".to_owned(),
        code: "KeyA".to_owned(),
        pressed,
        repeat: pressed,
        modifiers,
    }
}

#[test]
fn a_platform_key_reaches_the_listener_with_its_code_repeat_and_each_modifier() {
    let one = |pick: fn(&mut Modifiers)| {
        let mut modifiers = Modifiers::default();
        pick(&mut modifiers);
        modifiers
    };
    let cases = [
        one(|m| m.shift = true),
        one(|m| m.control = true),
        one(|m| m.alt = true),
        one(|m| m.meta = true),
    ];
    for modifiers in cases {
        // Given
        let (mut app, seen) = page();
        let platform = Platform::detached();

        // When
        for pressed in [true, false] {
            let _ = app.give(&key(pressed, modifiers), &platform);
        }

        // Then
        let held = DomModifiers {
            shift: modifiers.shift,
            control: modifiers.control,
            alt: modifiers.alt,
            meta: modifiers.meta,
        };
        let expected = [("keydown", true), ("keyup", false)].map(|(name, repeat)| {
            (
                name.to_owned(),
                "a".to_owned(),
                "KeyA".to_owned(),
                repeat,
                held,
            )
        });
        assert_eq!(*seen.borrow(), expected, "{modifiers:?}");
    }
}

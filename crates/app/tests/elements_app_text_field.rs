//! The focused text field and the platform's text input, through
//! `ElementsApp`: the app sets the field for the platform to mirror
//! after events and frames, only when it changed; the text input's
//! edits (`Event::TextEdited`) and pastes (`Event::PastedText`) reach
//! the field; an edit against a stale text is refused and the field set
//! again, so the mirror is corrected.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_platform::{App, Event, Platform, TextField};
use vapors_text::FontSystem;

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

fn edited(before: &str, text: &str, at: u32, composing: Option<(u32, u32)>) -> Event {
    Event::TextEdited {
        before: before.to_owned(),
        text: text.to_owned(),
        selection: (at, at),
        composing,
    }
}

#[test]
fn focus_in_a_text_field_sets_it_for_the_platform_once() {
    // Given
    let (doc, _) = field();
    let mut app = ElementsApp::new(Rc::clone(&doc));
    let platform = Platform::detached();

    // When
    click_field(&mut app, &platform);
    let set = platform.take_text_field();
    app.event(Event::PointerMoved { x: 151.0, y: 10.0 }, &platform);

    // Then: the field as it is, then nothing new after an event that changed nothing
    let Some(Some(TextField {
        text, selection, ..
    })) = set
    else {
        panic!("{set:?}");
    };
    assert_eq!((text.as_str(), selection), ("Warm", (4, 4)));
    assert_eq!(platform.take_text_field(), None);
}

#[test]
fn a_composing_edit_from_the_text_input_reaches_the_field() {
    // Given
    let (doc, input) = field();
    let mut app = ElementsApp::new(Rc::clone(&doc));
    let platform = Platform::detached();
    click_field(&mut app, &platform);
    drop(platform.take_text_field());

    // When
    app.event(edited("Warm", "Warmth", 6, Some((4, 6))), &platform);

    // Then: the field holds it, and the mirror is told
    assert_eq!(doc.borrow().value(input).as_deref(), Some("Warmth"));
    let set = platform.take_text_field();
    assert!(
        matches!(&set, Some(Some(field)) if field.text == "Warmth"),
        "{set:?}"
    );
}

#[test]
fn a_stale_edit_is_refused_and_the_field_set_again() {
    // Given: the field mirrored as "Warm"
    let (doc, input) = field();
    let mut app = ElementsApp::new(Rc::clone(&doc));
    let platform = Platform::detached();
    click_field(&mut app, &platform);
    drop(platform.take_text_field());

    // When: an edit made against "Cold"
    app.event(edited("Cold", "Colder", 6, None), &platform);

    // Then: the field kept its text, and the mirror is set to it again
    assert_eq!(doc.borrow().value(input).as_deref(), Some("Warm"));
    let set = platform.take_text_field();
    assert!(
        matches!(&set, Some(Some(field)) if field.text == "Warm"),
        "{set:?}"
    );
}

#[test]
fn a_page_paste_reaches_the_focused_field() {
    // Given
    let (doc, input) = field();
    let mut app = ElementsApp::new(Rc::clone(&doc));
    let platform = Platform::detached();
    click_field(&mut app, &platform);

    // When
    app.event(Event::PastedText("th".into()), &platform);

    // Then
    assert_eq!(doc.borrow().value(input).as_deref(), Some("Warmth"));
}

#[test]
fn leaving_the_field_unsets_it() {
    // Given
    let (doc, _) = field();
    let mut app = ElementsApp::new(Rc::clone(&doc));
    let platform = Platform::detached();
    click_field(&mut app, &platform);
    drop(platform.take_text_field());

    // When: a press below the field blurs it
    for event in [
        Event::PointerMoved { x: 150.0, y: 80.0 },
        Event::PrimaryButton {
            pressed: true,
            clicks: std::num::NonZeroU32::MIN,
        },
        Event::PrimaryButton {
            pressed: false,
            clicks: std::num::NonZeroU32::MIN,
        },
    ] {
        app.event(event, &platform);
    }

    // Then
    assert_eq!(platform.take_text_field(), Some(None));
}

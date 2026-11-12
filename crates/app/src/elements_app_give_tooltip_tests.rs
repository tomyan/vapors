//! A tooltip change asks for a frame: a hide is drawn, and a new
//! deadline reaches the frame's `Next`.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};
use vapors_platform::{Event, Instant, Modifiers, Next, Platform};
use vapors_text::FontSystem;

use super::ElementsApp;

/// An app showing one 100x20 button titled "Open (o)" at the top left.
fn app() -> (ElementsApp, Rc<RefCell<Document>>, Platform) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "body { margin: 0; display: flex; } button { width: 100px; height: 20px; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    let (body, open) = (doc.body(), doc.create_element("button"));
    doc.set_attribute(open, "title", "Open (o)");
    doc.append(body, open);
    let viewport = Viewport {
        width: 200.0,
        height: 100.0,
        scale: 1.0,
    };
    doc.update(viewport).expect("updates");
    let doc = Rc::new(RefCell::new(doc));
    (ElementsApp::new(Rc::clone(&doc)), doc, Platform::detached())
}

fn shift() -> Event {
    Event::Key {
        key: "Shift".into(),
        code: "ShiftLeft".into(),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::default(),
    }
}

/// `event` given, then whether it asked for a frame and what the
/// frame after it would wait for.
fn after(app: &mut ElementsApp, platform: &Platform, event: &Event) -> (bool, Next) {
    app.give(event, platform).expect("placed");
    (platform.take_redraw_request(), app.next_after_frame())
}

/// The pointer over the button and its tooltip shown by a frame at its
/// deadline, every request for a frame so far answered.
fn shown(app: &mut ElementsApp, doc: &Rc<RefCell<Document>>, platform: &Platform) -> Instant {
    let (_, next) = after(app, platform, &Event::PointerMoved { x: 10.0, y: 10.0 });
    let Next::WaitUntil(deadline) = next else {
        panic!("armed, the frame waits until the deadline: {next:?}");
    };
    assert!(app.input.tooltip(&doc.borrow(), deadline).is_some());
    assert_eq!(app.next_after_frame(), Next::Wait);
    deadline
}

#[test]
fn a_move_that_arms_again_after_a_key_hid_it_asks_for_a_frame_and_waits_for_the_deadline() {
    // Given
    let (mut app, doc, platform) = app();
    shown(&mut app, &doc, &platform);

    // When
    let hidden = after(&mut app, &platform, &shift());
    let rearmed = after(
        &mut app,
        &platform,
        &Event::PointerMoved { x: 11.0, y: 10.0 },
    );

    // Then
    assert_eq!(hidden, (true, Next::Wait));
    assert!(rearmed.0, "the re-arm asks for a frame");
    assert!(matches!(rearmed.1, Next::WaitUntil(_)), "{:?}", rearmed.1);
}

#[test]
fn a_blur_that_hides_a_shown_tooltip_asks_for_a_frame() {
    // Given
    let (mut app, doc, platform) = app();
    shown(&mut app, &doc, &platform);

    // When
    let blurred = after(&mut app, &platform, &Event::WindowFocus { focused: false });

    // Then
    assert!(blurred.0);
}

#[test]
fn a_hide_before_the_deadline_replaces_the_frames_wait_for_it() {
    // Given: armed, the last frame waiting until its deadline
    let (mut app, _doc, platform) = app();
    let (_, armed) = after(
        &mut app,
        &platform,
        &Event::PointerMoved { x: 10.0, y: 10.0 },
    );

    // When
    let hidden = after(&mut app, &platform, &shift());

    // Then: a frame now, which waits for nothing
    assert!(matches!(armed, Next::WaitUntil(_)), "{armed:?}");
    assert_eq!(hidden, (true, Next::Wait));
}

#[test]
fn a_show_in_a_frame_asks_for_no_further_frame_and_no_later_event_asks_for_one() {
    // Given: armed, every frame request answered
    let (mut app, doc, platform) = app();
    let (_, next) = after(
        &mut app,
        &platform,
        &Event::PointerMoved { x: 10.0, y: 10.0 },
    );
    let Next::WaitUntil(deadline) = next else {
        panic!("armed: {next:?}");
    };

    // When: a frame's update at the deadline shows it
    assert!(app.input.tooltip(&doc.borrow(), deadline).is_some());
    let next = app.next_after_frame();
    let decided = app.last_decision();
    let moved = after(
        &mut app,
        &platform,
        &Event::PointerMoved { x: 11.0, y: 10.0 },
    );

    // Then
    assert!(decided.tooltip_changed);
    assert_eq!(next, Next::Wait);
    assert_eq!(moved, (false, Next::Wait));
}

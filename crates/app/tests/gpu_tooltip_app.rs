//! GPU tier: `ElementsApp` draws the tooltip over its frame from the
//! frame at its deadline, stops drawing it on the frame after the event
//! that hides it, and draws a frame with no tooltip as the painter
//! alone draws it.
#[path = "support/surface.rs"]
mod support;
#[path = "support/tooltip.rs"]
mod tooltip;

use std::cell::RefCell;
use std::rc::Rc;

use support::{Rig, app_frame_at, document, update};
use tooltip::{GREEN, TOOLTIP, bounds};
use vapors_app::ElementsApp;
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_platform::{Event, Instant, Modifiers, Next, Platform};

/// A blue 10px square titled "." at the top left, under `TOOLTIP`.
fn titled_square() -> Rc<RefCell<Document>> {
    let css = ".open { position: absolute; left: 0; top: 0; width: 10px; height: 10px; \
        background-color: #0000ff; }";
    let mut doc = document(&format!("{TOOLTIP} {css}"), &[]);
    let (body, square) = (doc.body(), doc.create_element("div"));
    doc.set_attribute(square, "class", "open");
    doc.set_attribute(square, "title", ".");
    doc.append(body, square);
    update(&mut doc, 1.0);
    Rc::new(RefCell::new(doc))
}

fn escape() -> Event {
    Event::Key {
        key: "Escape".into(),
        code: "Escape".into(),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::default(),
    }
}

#[test]
fn gpu_the_tooltip_draws_from_its_deadline_until_the_frame_after_it_hides() {
    // Given
    let rig = Rig::new();
    let mut app = ElementsApp::new(titled_square());
    let platform = Platform::detached();
    let moved = Event::PointerMoved { x: 2.0, y: 2.0 };
    app.give(&moved, &platform).expect("placed");
    let (Next::WaitUntil(deadline), before) = app_frame_at(&mut app, &rig.gpu, Instant::now())
    else {
        panic!("the frame after the move waits for the tooltip's deadline");
    };

    // When
    let (_, shown) = app_frame_at(&mut app, &rig.gpu, deadline);
    app.give(&escape(), &platform).expect("placed");
    let (_, hidden) = app_frame_at(&mut app, &rig.gpu, deadline);

    // Then
    assert_eq!(bounds(&before, GREEN), None, "nothing before the deadline");
    let [left, top, ..] = bounds(&shown, GREEN).expect("shown at the deadline");
    assert_eq!([left, top], [12, 17]);
    assert_eq!(hidden, before, "the frame after Esc draws no tooltip");
}

#[test]
fn gpu_a_shown_tooltip_stays_where_it_showed_when_the_pointer_moves_within_its_element() {
    // Given: shown at the pointer (2, 2)
    let rig = Rig::new();
    let mut app = ElementsApp::new(titled_square());
    let platform = Platform::detached();
    let moved = Event::PointerMoved { x: 2.0, y: 2.0 };
    app.give(&moved, &platform).expect("placed");
    let (Next::WaitUntil(deadline), _) = app_frame_at(&mut app, &rig.gpu, Instant::now()) else {
        panic!("the frame after the move waits for the tooltip's deadline");
    };
    app_frame_at(&mut app, &rig.gpu, deadline);

    // When: 5 px right, still over the square, and a frame drawn
    let within = Event::PointerMoved { x: 7.0, y: 2.0 };
    app.give(&within, &platform).expect("placed");
    let (_, moved_frame) = app_frame_at(&mut app, &rig.gpu, deadline);

    // Then
    let [left, top, ..] = bounds(&moved_frame, GREEN).expect("still shown");
    assert_eq!([left, top], [12, 17]);
}

#[test]
fn gpu_a_frame_with_no_tooltip_draws_what_the_painter_alone_draws() {
    // Given: the pointer moved, off the title
    let doc = titled_square();
    let mut rig = Rig::new();
    let mut app = ElementsApp::new(Rc::clone(&doc));
    let moved = Event::PointerMoved { x: 20.0, y: 20.0 };
    app.give(&moved, &Platform::detached()).expect("placed");

    // When
    let (_, framed) = app_frame_at(&mut app, &rig.gpu, Instant::now());

    // Then
    assert_eq!(framed, rig.draw(&doc.borrow()).expect("draws"));
}

#[test]
fn gpu_a_bad_tooltip_style_fails_the_first_frame_with_no_hover() {
    // Given: a background that is not a colour, and the pointer never moved
    let doc = document(":root { --tooltip-background: 12px; }", &[]);
    let rig = Rig::new();
    let mut app = ElementsApp::new(Rc::new(RefCell::new(doc)));

    // When
    let (next, _) = app_frame_at(&mut app, &rig.gpu, Instant::now());

    // Then
    assert!(matches!(next, Next::Exit));
    let failure = app.failure().get().expect("a failure");
    assert!(
        failure.contains("`--tooltip-background: 12px`"),
        "{failure}"
    );
}

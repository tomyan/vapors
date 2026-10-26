//! A drag through `ElementsApp::event` (3-E1a): every `pointermove` runs
//! its listener at once, but layout and paint wait for the frame's
//! Update phase, so twenty moves between frames are one update, laid out
//! at the last move (the lead's condition on dispatching input as it
//! arrives, design 02).

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_dom::{Dom, Event as DomEvent, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_platform::{App, Event, Platform};
use vapors_text::FontSystem;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

/// A 20x20 thumb at the top left that captures on pointerdown and, on
/// each move, sets its own width to the pointer's x.
fn thumb() -> (Rc<RefCell<Document>>, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } div { width: 20px; height: 20px; }",
    )
    .expect("parses");
    let body = doc.body();
    let thumb = doc.create_element("div");
    doc.append(body, thumb);
    doc.listen(
        thumb,
        "pointerdown",
        Box::new(|e: &mut DomEvent| e.set_pointer_capture()),
    );
    doc.update(VIEWPORT).expect("updates");
    let doc = Rc::new(RefCell::new(doc));
    let writer = Rc::downgrade(&doc);
    doc.borrow_mut().listen(
        thumb,
        "pointermove",
        Box::new(move |e: &mut DomEvent| {
            let doc = writer.upgrade().expect("the document lives");
            let style = format!("width: {}px", e.client()[0]);
            doc.borrow_mut().set_attribute(thumb, "style", &style);
        }),
    );
    (doc, thumb)
}

fn width(doc: &Rc<RefCell<Document>>, node: NodeId) -> f32 {
    let doc = doc.borrow();
    doc.painted()
        .and_then(|painted| painted.border_box(node))
        .expect("laid out")
        .width
}

#[test]
fn twenty_moves_between_frames_are_one_update_at_the_last_move() {
    // Given: pressed on the thumb
    let (doc, thumb) = thumb();
    let mut app = ElementsApp::new(Rc::clone(&doc)).with_scale(1.0);
    let platform = Platform::detached();
    app.event(Event::PointerMoved { x: 5.0, y: 5.0 }, &platform);
    app.event(Event::PrimaryButton { pressed: true }, &platform);

    // When: twenty moves, the last at x 125, with no frame between them
    for step in 1..=20_u8 {
        app.event(
            Event::PointerMoved {
                x: 25.0 + f32::from(step) * 5.0,
                y: 50.0,
            },
            &platform,
        );
    }

    // Then: nothing was laid out during the moves, a frame is asked for,
    // and that one update lays out the last move's width
    assert_eq!(width(&doc, thumb), 20.0);
    assert!(platform.take_redraw_request());
    doc.borrow_mut().update(VIEWPORT).expect("updates");
    assert_eq!(width(&doc, thumb), 125.0);
}

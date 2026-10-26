//! Touches through `ElementsApp::event` (3-E1b): a pan scrolls the list
//! and cancels the touch on an item no listener captured, but a touch a
//! `pointerdown` listener captured (a slider's thumb) owns the gesture,
//! so its pans do not scroll and its moves reach the thumb. The rule is
//! RELAXED until 3-E4's `touch-action`.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_dom::{Dom, Event as DomEvent, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_platform::{App, Event, Platform, TouchPhase};
use vapors_text::FontSystem;

type Log = Rc<RefCell<Vec<String>>>;

/// A 100 px list (300 px of content) holding `.thumb` (y 0..20), which
/// captures on pointerdown, and `.item` (y 20..300); both log events.
fn list() -> (Rc<RefCell<Document>>, NodeId, Log) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } .list { height: 100px; overflow: auto; } .thumb { height: 20px; } .item { height: 280px; }",
    )
    .expect("parses");
    let log = Log::default();
    let list = add(&mut doc, None, "list");
    for class in ["thumb", "item"] {
        let node = add(&mut doc, Some(list), class);
        for event in ["pointermove", "pointercancel"] {
            let (log, label) = (Rc::clone(&log), class.to_owned());
            doc.listen(
                node,
                event,
                Box::new(move |e: &mut DomEvent| {
                    log.borrow_mut().push(format!("{label} {}", e.name()));
                }),
            );
        }
        if class == "thumb" {
            doc.listen(
                node,
                "pointerdown",
                Box::new(|e: &mut DomEvent| e.set_pointer_capture()),
            );
        }
    }
    doc.update(Viewport {
        width: 200.0,
        height: 200.0,
        scale: 1.0,
    })
    .expect("updates");
    (Rc::new(RefCell::new(doc)), list, log)
}

fn add(doc: &mut Document, parent: Option<NodeId>, class: &str) -> NodeId {
    let node = doc.create_element("div");
    doc.set_attribute(node, "class", class);
    let parent = parent.unwrap_or_else(|| doc.body());
    doc.append(parent, node);
    node
}

/// One finger down at y `from`, dragged up 30 px with its pans.
fn drag(app: &mut ElementsApp, from: f32) {
    let platform = Platform::detached();
    let touch = |phase, y| Event::Touch {
        id: 1,
        phase,
        x: 10.0,
        y,
    };
    app.event(touch(TouchPhase::Started, from), &platform);
    for step in 1..=3_u8 {
        let y = from - f32::from(step) * 10.0;
        app.event(touch(TouchPhase::Moved, y), &platform);
        app.event(Event::Pan { dx: 0.0, dy: -10.0 }, &platform);
    }
}

#[test]
fn a_pan_scrolls_the_list_and_cancels_an_uncaptured_touch() {
    // Given
    let (doc, list, log) = list();
    let mut app = ElementsApp::new(Rc::clone(&doc)).with_scale(1.0);

    // When: on the item
    drag(&mut app, 60.0);

    // Then: the list scrolled; the item heard one move, then the cancel
    assert_eq!(doc.borrow().scroll_offset(list).expect("scrolls")[1], 30.0);
    assert_eq!(*log.borrow(), ["item pointermove", "item pointercancel"]);
}

#[test]
fn a_captured_touch_owns_its_pans() {
    // Given
    let (doc, list, log) = list();
    let mut app = ElementsApp::new(Rc::clone(&doc)).with_scale(1.0);

    // When: on the thumb
    drag(&mut app, 15.0);

    // Then: nothing scrolled, and the thumb heard every move
    assert_eq!(doc.borrow().scroll_offset(list).expect("scrolls")[1], 0.0);
    assert_eq!(*log.borrow(), ["thumb pointermove"; 3]);
}

#[test]
fn a_pan_cancels_an_uncaptured_touch_even_when_nothing_scrolls() {
    // Given: a list already at its end, so a pan up moves nothing
    let (doc, list, log) = list();
    doc.borrow_mut().scroll_node_by(list, [0.0, 200.0]);
    let mut app = ElementsApp::new(Rc::clone(&doc)).with_scale(1.0);
    let platform = Platform::detached();

    // When: a finger on the item pans the list further down
    app.event(
        Event::Touch {
            id: 1,
            phase: TouchPhase::Started,
            x: 10.0,
            y: 60.0,
        },
        &platform,
    );
    app.event(Event::Pan { dx: 0.0, dy: -10.0 }, &platform);

    // Then: as Chromium does, the pan takes the touch (capture.events)
    assert_eq!(*log.borrow(), ["item pointercancel"]);
}

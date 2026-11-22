//! GPU tier: the app fires a scroll container's `scroll` once per frame,
//! before the update (virtual list V1, `docs/plans/virtual-list.md`):
//! wheel events between two frames are one `scroll` in the next, and a
//! frame with no scroll fires none and asks for no other.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_dom::{Dom, Event as DomEvent};
use vapors_elements::{Document, Viewport};
use vapors_gpu_harness::gpu;
use vapors_platform::{App, Event, Frame, Instant, Platform};
use vapors_renderer::clear;
use vapors_scene::Color;
use vapors_text::FontSystem;

type Heard = Rc<RefCell<Vec<Option<[f32; 2]>>>>;

/// A 64 x 32 list holding four 20 px rows at the top left, and the
/// offsets its `scroll`s carried.
fn list() -> (Rc<RefCell<Document>>, Heard) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } .list { width: 64px; height: 32px; overflow: auto; } \
         .row { height: 20px; }",
    )
    .expect("parses");
    let body = doc.body();
    let list = doc.create_element("div");
    doc.set_attribute(list, "class", "list");
    doc.append(body, list);
    for _ in 0..4 {
        let row = doc.create_element("div");
        doc.set_attribute(row, "class", "row");
        doc.append(list, row);
    }
    let heard = Rc::new(RefCell::new(Vec::new()));
    let into = Rc::clone(&heard);
    doc.listen(
        list,
        "scroll",
        Box::new(move |event: &mut DomEvent| into.borrow_mut().push(event.scroll_offset())),
    );
    doc.update(Viewport {
        width: 64.0,
        height: 64.0,
        scale: 1.0,
    })
    .expect("updates");
    (Rc::new(RefCell::new(doc)), heard)
}

#[test]
fn gpu_two_wheel_events_between_frames_fire_one_scroll_and_then_none() {
    // Given: a frame drawn with the pointer over the list
    let (doc, heard) = list();
    let mut app = ElementsApp::new(Rc::clone(&doc));
    let platform = Platform::detached();
    let gpu = gpu();
    let texture = clear(&gpu.device, &gpu.queue, 64, 64, Color::default());
    let frame = |app: &mut ElementsApp| {
        app.frame(&Frame {
            adapter: &gpu.adapter,
            device: &gpu.device,
            queue: &gpu.queue,
            texture: &texture,
            index: 0,
            scale_factor: 1.0,
            now: Instant::now(),
            resumed: false,
        });
    };
    app.event(Event::PointerMoved { x: 10.0, y: 10.0 }, &platform);
    frame(&mut app);

    // When: two wheel events, a frame, then a frame with nothing scrolled
    for _ in 0..2 {
        app.event(Event::Scroll { dx: 0.0, dy: 15.0 }, &platform);
    }
    let before = heard.borrow().clone();
    frame(&mut app);
    let after_one = heard.borrow().clone();
    frame(&mut app);
    let idle = app.last_decision();

    // Then
    assert_eq!(
        (before, after_one, heard.borrow().clone()),
        (vec![], vec![Some([0.0, 30.0])], vec![Some([0.0, 30.0])])
    );
    assert!(!idle.document_changed, "{idle:?}");
}

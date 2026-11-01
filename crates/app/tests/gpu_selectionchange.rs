//! GPU tier: the app fires a text field's `selectionchange` once per
//! frame (E6a), as Chromium queues one per task: two moves of the caret
//! between frames are one, and a frame with no move fires none.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_dom::{Dom, Event as DomEvent};
use vapors_elements::{Document, Viewport};
use vapors_gpu_harness::gpu;
use vapors_platform::{App, Event, Frame, Instant, Modifiers, Platform};
use vapors_renderer::clear;
use vapors_scene::Color;
use vapors_text::FontSystem;

/// A 160 px field holding "Warm" at the top left, and a count of its
/// `selectionchange`s.
fn field() -> (Rc<RefCell<Document>>, Rc<RefCell<u32>>) {
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
    let heard = Rc::new(RefCell::new(0));
    let into = Rc::clone(&heard);
    doc.listen(
        input,
        "selectionchange",
        Box::new(move |_: &mut DomEvent| *into.borrow_mut() += 1),
    );
    doc.update(Viewport {
        width: 64.0,
        height: 64.0,
        scale: 1.0,
    })
    .expect("updates");
    (Rc::new(RefCell::new(doc)), heard)
}

fn key(app: &mut ElementsApp, platform: &Platform, key: &str, pressed: bool) {
    let event = Event::Key {
        key: key.to_owned(),
        pressed,
        repeat: false,
        modifiers: Modifiers::default(),
    };
    app.event(event, platform);
}

#[test]
fn gpu_two_caret_moves_between_frames_fire_one_selectionchange() {
    // Given: the field clicked into, and a frame drawn since
    let (doc, heard) = field();
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
        });
    };
    for event in [
        Event::PointerMoved { x: 60.0, y: 10.0 },
        Event::PrimaryButton { pressed: true },
        Event::ClickCount(std::num::NonZeroU32::MIN),
        Event::PrimaryButton { pressed: false },
    ] {
        app.event(event, &platform);
    }
    frame(&mut app);
    *heard.borrow_mut() = 0;

    // When: two moves, a frame, then a frame with nothing moved
    for _ in 0..2 {
        key(&mut app, &platform, "ArrowLeft", true);
        key(&mut app, &platform, "ArrowLeft", false);
    }
    let before = *heard.borrow();
    frame(&mut app);
    let after_one = *heard.borrow();
    frame(&mut app);

    // Then
    assert_eq!((before, after_one, *heard.borrow()), (0, 1, 1));
}

//! GPU tier: a finger flicking a list through the platform's `Gestures`
//! and `ElementsApp`, as the window delivers it: the list follows the
//! finger, asks for a frame when the finger lifts, coasts on at each
//! frame, and asks for none once the coast ends.

#![allow(clippy::cast_precision_loss, reason = "a few frames and milliseconds")]

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_app::ElementsApp;
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;
use vapors_gpu_harness::{Gpu, gpu};
use vapors_platform::{App, Event, Frame, Gestures, Instant, Next, Platform, Start, TouchPhase};
use vapors_renderer::clear;
use vapors_scene::Color;
use vapors_text::FontSystem;

const SIZE: u32 = 300;

/// A 300 px list of 5000 px at the top left, scale 1.
fn page() -> (Rc<RefCell<Document>>, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css =
        "body { margin: 0; } .list { height: 300px; overflow: auto; } .content { height: 5000px; }";
    doc.add_stylesheet("flick.css", css).expect("parses");
    let (body, list, content) = (
        doc.body(),
        doc.create_element("div"),
        doc.create_element("div"),
    );
    doc.set_attribute(list, "class", "list");
    doc.set_attribute(content, "class", "content");
    doc.append(body, list);
    doc.append(list, content);
    (Rc::new(RefCell::new(doc)), list)
}

fn frame(app: &mut ElementsApp, gpu: &Gpu, now: Instant) -> Next {
    let texture = clear(&gpu.device, &gpu.queue, SIZE, SIZE, Color::default());
    app.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 0,
        scale_factor: 1.0,
        now,
    })
}

/// The touch and the gestures it completes, delivered as the window
/// does: the touch first.
fn deliver(
    app: &mut ElementsApp,
    gestures: &mut Gestures,
    platform: &Platform,
    touch: (TouchPhase, f32, u64),
) {
    let (phase, y, ms) = touch;
    app.event(finger(phase, y), platform);
    for gesture in gestures.touch(0, phase, 50.0, y, Duration::from_millis(ms)) {
        app.event(gesture, platform);
    }
}

fn finger(phase: TouchPhase, y: f32) -> Event {
    Event::Touch {
        id: 0,
        phase,
        x: 50.0,
        y,
    }
}

/// An app showing `doc`, started and drawn once at `at`.
fn started(gpu: &Gpu, doc: &Rc<RefCell<Document>>, at: Instant) -> ElementsApp {
    let mut app = ElementsApp::new(Rc::clone(doc));
    let start = Start {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        format: wgpu::TextureFormat::Rgba8Unorm,
        width: SIZE,
        height: SIZE,
        scale_factor: 1.0,
    };
    app.start(&start).expect("starts");
    frame(&mut app, gpu, at);
    app
}

#[test]
fn gpu_a_flicked_list_coasts_after_the_finger_lifts_and_then_rests() {
    // Given a started app and a finger dragging up at 1000 px/s
    let (gpu, (doc, list)) = (gpu(), page());
    let (platform, mut gestures, base) = (Platform::detached(), Gestures::new(), Instant::now());
    let mut app = started(&gpu, &doc, base);
    deliver(
        &mut app,
        &mut gestures,
        &platform,
        (TouchPhase::Started, 250.0, 0),
    );
    for n in 1..=10_u64 {
        deliver(
            &mut app,
            &mut gestures,
            &platform,
            (TouchPhase::Moved, 250.0 - 8.0 * n as f32, 8 * n),
        );
    }
    let dragged = doc.borrow().scroll_offset(list).expect("scrolls")[1];
    app.event(finger(TouchPhase::Ended, 170.0), &platform);
    let _ = platform.take_redraw_request();

    // When the finger's lift becomes a fling
    let fling = gestures.touch(0, TouchPhase::Ended, 50.0, 170.0, Duration::from_millis(88));
    assert!(matches!(fling[..], [Event::Fling { .. }]), "{fling:?}");
    app.event(fling[0].clone(), &platform);

    // Then the fling asks for a frame, the list coasts, and rests
    assert!(
        platform.take_redraw_request(),
        "the fling asked for no frame"
    );
    let nexts: Vec<Next> = (1..=90)
        .map(|n| frame(&mut app, &gpu, base + Duration::from_millis(88 + 16 * n)))
        .collect();
    let coasted = doc.borrow().scroll_offset(list).expect("scrolls")[1] - dragged;
    assert!(
        (150.0..=250.0).contains(&coasted),
        "coasted {coasted} px after an 80 px drag"
    );
    assert_eq!(nexts[0], Next::Continue);
    assert_eq!(nexts.last(), Some(&Next::Wait));
}

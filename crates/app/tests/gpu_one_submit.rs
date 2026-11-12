//! GPU tier: an `ElementsApp` frame records the scene and the tooltip
//! over it into one command encoder and submits it once, however many
//! draws it holds (docs/perf/baseline-2026-11-10.md, finding 1: a
//! submit costs about 70 µs on ANV).

#[path = "support/surface.rs"]
mod support;
#[path = "support/tooltip.rs"]
mod tooltip;

use std::cell::RefCell;
use std::rc::Rc;

use support::{SIZE, document, update};
use tooltip::TOOLTIP;
use vapors_app::ElementsApp;
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_gpu_harness::{Gpu, gpu};
use vapors_platform::{App, Event, Frame, Instant, Next, Platform};
use vapors_renderer::clear;
use vapors_scene::Color;

/// Shadowed, bordered squares with text, one titled: shadow, quad and
/// text runs, several of each, and the tooltip's quad and text.
fn busy_page() -> Rc<RefCell<Document>> {
    let css = ".row { width: 8px; height: 4px; border: 1px solid #ff0000; \
        box-shadow: 1px 1px 2px #000000; font-size: 4px; }";
    let mut doc = document(&format!("{TOOLTIP} {css}"), &[]);
    let body = doc.body();
    for label in ["a", "b", "c"] {
        let row = doc.create_element("div");
        doc.set_attribute(row, "class", "row");
        doc.set_attribute(row, "title", ".");
        let text = doc.create_text(label);
        doc.append(row, text);
        doc.append(body, row);
    }
    update(&mut doc, 1.0);
    Rc::new(RefCell::new(doc))
}

/// The queue's next submission's index, less one. wgpu keeps the
/// number private; its `Debug` form, `SubmissionIndex { index: N }`,
/// is the only reading, so any other form fails here, naming it.
fn last_submission(gpu: &Gpu) -> u64 {
    let index = format!("{:?}", gpu.queue.submit([]));
    let digits = index
        .strip_prefix("SubmissionIndex { index: ")
        .and_then(|rest| rest.strip_suffix(" }"))
        .unwrap_or_else(|| panic!("unrecognised SubmissionIndex form: {index}"));
    digits.parse().expect("a submission number")
}

/// The submits `app`'s frame at `now` made.
fn submits_in_frame(app: &mut ElementsApp, gpu: &Gpu, now: Instant) -> (Next, u64) {
    let texture = clear(&gpu.device, &gpu.queue, SIZE, SIZE, Color::default());
    let before = last_submission(gpu);
    let next = app.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 0,
        scale_factor: 1.0,
        now,
    });
    (next, last_submission(gpu) - before - 1)
}

#[test]
fn gpu_a_frame_with_many_draws_and_a_tooltip_submits_once() {
    // Given: the frame after a pointer move waits for the tooltip
    let gpu = gpu();
    let mut app = ElementsApp::new(busy_page());
    let moved = Event::PointerMoved { x: 2.0, y: 2.0 };
    app.give(&moved, &Platform::detached()).expect("placed");
    let (Next::WaitUntil(deadline), _) = submits_in_frame(&mut app, &gpu, Instant::now()) else {
        panic!("the frame after the move waits for the tooltip's deadline");
    };

    // When
    let (_, submits) = submits_in_frame(&mut app, &gpu, deadline);

    // Then
    assert!(
        app.frame_counts().total().draw_calls > 6,
        "too few draws to tell"
    );
    assert_eq!(submits, 1);
}

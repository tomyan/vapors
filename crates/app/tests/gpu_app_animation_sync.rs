//! GPU tier: a `stop_animation` made while a frame runs, after its
//! paint, takes effect in that frame's decision (docs/gpu-surface.md).

#[path = "support/surface.rs"]
mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{RED, Solid, app_frame, document};
use vapors_app::ElementsApp;
use vapors_gpu_harness::gpu;
use vapors_platform::Next;

#[test]
fn gpu_a_stop_after_the_paint_in_a_frame_ends_that_frames_request_for_the_next() {
    // Given: animating, and a frame-times hook (it runs after the
    // paint, before the app decides) that stops the animation
    let css = ".track { width: 16px; height: 16px; }";
    let elements = [(None, "gpu-surface", "track")];
    let mut app = ElementsApp::new(Rc::new(RefCell::new(document(css, &elements))));
    let (solid, log) = Solid::new(RED);
    let track = app.mount("track", Box::new(solid)).expect("mounts");
    let stopping = Rc::new(RefCell::new(false));
    let (stop, hook_track) = (Rc::clone(&stopping), track.clone());
    let mut app = app.with_frame_times(Box::new(move |_| {
        if *stop.borrow() {
            hook_track.stop_animation().expect("mounted");
        }
    }));
    let gpu = gpu();
    app_frame(&mut app, &gpu);
    track.request_animation().expect("mounted");
    assert_eq!(app_frame(&mut app, &gpu).0, Next::Continue);

    // When
    *stopping.borrow_mut() = true;
    let stopped_in_frame = app_frame(&mut app, &gpu).0;

    // Then
    assert_eq!(stopped_in_frame, Next::Wait);
    assert_eq!(log.borrow().frames.len(), 3, "the first, and two animating");
}

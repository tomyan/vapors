//! GPU tier: an `ElementsApp` frame with a surface renderer mounted
//! before the painter existed (docs/gpu-surface.md): the first frame
//! renders it, an idle frame asks for none, an invalidate through its
//! handle renders it once more, a renderer unmounted after the first
//! frame is handed back, and a failed frame closes the app's handles.

#[path = "support/surface.rs"]
mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{RED, Solid, app_frame, document, pixel};
use vapors_app::{ElementsApp, InvalidateError};
use vapors_gpu_harness::{Gpu, gpu};
use vapors_platform::Next;

const TRACK: &str = ".track { width: 16px; height: 16px; }";

/// An app showing one 16 x 16 surface named `track`, and its GPU.
fn rig() -> (ElementsApp, Gpu) {
    let doc = document(TRACK, &[(None, "gpu-surface", "track")]);
    (ElementsApp::new(Rc::new(RefCell::new(doc))), gpu())
}

#[test]
fn gpu_a_renderer_mounted_before_the_first_frame_draws_in_it_and_an_idle_app_then_waits() {
    // Given
    let (mut app, gpu) = rig();
    let (solid, log) = Solid::new(RED);
    app.mount("track", Box::new(solid)).expect("mounts");

    // When: the first frame also takes the document's building
    let (_, rgba) = app_frame(&mut app, &gpu);
    let (idle, _) = app_frame(&mut app, &gpu);

    // Then
    assert_eq!(pixel(&rgba, 4, 4), RED);
    assert!(matches!(idle, Next::Wait), "an idle app asked for a frame");
    assert_eq!(log.borrow().frames.len(), 1, "an idle frame rendered");
    assert!(app.failure().get().is_none());
}

#[test]
fn gpu_an_invalidate_through_the_handle_renders_the_surface_once_more() {
    // Given
    let (mut app, gpu) = rig();
    let (solid, log) = Solid::new(RED);
    let handle = app.mount("track", Box::new(solid)).expect("mounts");
    app_frame(&mut app, &gpu);

    // When
    handle.invalidate().expect("mounted");
    let (next, _) = app_frame(&mut app, &gpu);
    app_frame(&mut app, &gpu);

    // Then
    assert!(matches!(next, Next::Wait));
    assert_eq!(log.borrow().frames.len(), 2);
}

#[test]
fn gpu_a_renderer_unmounted_after_the_first_frame_is_handed_back() {
    // Given
    let (mut app, gpu) = rig();
    let (solid, log) = Solid::new(RED);
    let handle = app.mount("track", Box::new(solid)).expect("mounts");
    app_frame(&mut app, &gpu);

    // When
    let renderer = app.unmount("track").expect("mounted");
    drop(renderer);

    // Then
    assert!(log.borrow().dropped);
    assert_eq!(
        handle.invalidate(),
        Err(InvalidateError::NotMounted("track".into()))
    );
}

#[test]
fn gpu_a_handle_is_refused_once_a_frame_has_failed() {
    // Given: `track` is painted with no renderer, so the frame fails
    let (mut app, gpu) = rig();
    let (solid, _log) = Solid::new(RED);
    let handle = app.mount("other", Box::new(solid)).expect("mounts");

    // When
    let (next, _) = app_frame(&mut app, &gpu);

    // Then
    assert!(matches!(next, Next::Exit));
    assert_eq!(
        handle.invalidate(),
        Err(InvalidateError::LoopClosed("other".into()))
    );
}

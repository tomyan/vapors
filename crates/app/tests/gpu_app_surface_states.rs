//! GPU tier: what an `ElementsApp` reports about its surfaces and its
//! last frame's decision, for a failing check to print
//! (`surface_gradient --check`'s trace).

#[path = "support/surface.rs"]
mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{RED, Solid, app_frame, document};
use vapors_app::{ElementsApp, SurfaceHandle, SurfaceState};
use vapors_gpu_harness::gpu;

fn app_with_track() -> (ElementsApp, SurfaceHandle) {
    let elements = [(None, "gpu-surface", "track")];
    let css = ".track { width: 16px; height: 16px; }";
    let mut app = ElementsApp::new(Rc::new(RefCell::new(document(css, &elements))));
    let (solid, _) = Solid::new(RED);
    let handle = app.mount("track", Box::new(solid)).expect("mounts");
    (app, handle)
}

#[test]
fn gpu_the_app_reports_no_surfaces_before_its_first_frame() {
    // Given
    let (app, _) = app_with_track();

    // When
    let states = app.surface_states();

    // Then
    assert_eq!(states, []);
}

#[test]
fn gpu_the_app_reports_an_animating_painted_surface_after_a_frame() {
    // Given
    let gpu = gpu();
    let (mut app, track) = app_with_track();
    track.request_animation().expect("mounted");

    // When
    app_frame(&mut app, &gpu);

    // Then
    let state = SurfaceState {
        name: "track".into(),
        fresh: false,
        stale: false,
        animating: true,
        backing: true,
        waiting: false,
    };
    assert_eq!(app.surface_states(), [state]);
}

#[test]
fn gpu_the_app_reports_why_it_asked_for_the_next_frame() {
    // Given: animating, so the surface wants the next frame
    let gpu = gpu();
    let (mut app, track) = app_with_track();
    app_frame(&mut app, &gpu);
    track.request_animation().expect("mounted");

    // When
    app_frame(&mut app, &gpu);
    let animating = app.last_decision();
    track.stop_animation().expect("mounted");
    app_frame(&mut app, &gpu);
    let stopped = app.last_decision();

    // Then
    assert!(animating.surfaces_want && !animating.document_changed);
    assert!(!stopped.surfaces_want && !stopped.surface_invalidated);
}

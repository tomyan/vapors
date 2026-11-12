//! Renderers mounted on an `ElementsApp` before its painter exists
//! (docs/gpu-surface.md): a taken name is refused at `mount`, and a
//! `SurfaceHandle` invalidates its surface while `run` owns the app,
//! waking the loop through the waker the app kept from an event or a
//! drawn frame; before the loop starts it leaves the surface for the
//! first frame; it refuses a name no longer mounted and an app dropped.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::{
    ElementsApp, InvalidateError, Invalidated, MountError, SurfaceFrame, SurfaceRenderer,
};
use vapors_elements::Document;
use vapors_platform::{App, Event, Next, Platform};
use vapors_text::FontSystem;

/// Draws nothing; these tests never reach a GPU.
struct Blank;

impl SurfaceRenderer for Blank {
    fn render(&mut self, _frame: SurfaceFrame<'_>) -> Result<(), vapors_app::SurfaceError> {
        Ok(())
    }
}

fn app() -> ElementsApp {
    let document = Document::new(FontSystem::bundle_only());
    ElementsApp::new(Rc::new(RefCell::new(document)))
}

/// Takes the new document's own change, so the app is idle: Wait.
fn settle(app: &mut ElementsApp) {
    let _ = app.next_after_frame();
    assert!(matches!(app.next_after_frame(), Next::Wait));
}

#[test]
fn mounting_a_taken_name_before_the_first_frame_is_refused_at_mount() {
    // Given
    let mut app = app();
    app.mount("track", Box::new(Blank)).expect("mounts");

    // When
    let refused = app.mount("track", Box::new(Blank));

    // Then
    assert_eq!(
        refused.err(),
        Some(MountError::AlreadyMounted("track".into()))
    );
}

#[test]
fn a_name_unmounted_before_the_first_frame_can_be_mounted_again() {
    // Given
    let mut app = app();
    app.mount("track", Box::new(Blank)).expect("mounts");

    // When
    let unmounted = app.unmount("track");

    // Then
    assert!(unmounted.is_ok());
    assert!(app.mount("track", Box::new(Blank)).is_ok());
    assert_eq!(
        app.unmount("other").err(),
        Some(MountError::NotMounted("other".into()))
    );
}

#[test]
fn invalidating_through_a_handle_wakes_the_loop_the_app_was_given() {
    // Given: the app kept the platform's waker from an event
    let mut app = app();
    let handle = app.mount("track", Box::new(Blank)).expect("mounts");
    let platform = Platform::detached();
    app.event(Event::PointerLeft, &platform);
    let _ = platform.take_redraw_request();

    // When
    let invalidated = handle.clone().invalidate();

    // Then
    assert_eq!(invalidated, Ok(Invalidated::Woke));
    assert!(
        platform.take_redraw_request(),
        "the invalidate woke no frame"
    );
}

#[test]
fn an_invalidated_surface_asks_for_the_next_frame() {
    // Given
    let mut app = app();
    let handle = app.mount("track", Box::new(Blank)).expect("mounts");
    settle(&mut app);

    // When
    handle.invalidate().expect("mounted");

    // Then
    assert!(matches!(app.next_after_frame(), Next::Continue));
}

#[test]
fn invalidating_a_surface_no_longer_mounted_is_refused() {
    // Given
    let mut app = app();
    let handle = app.mount("track", Box::new(Blank)).expect("mounts");
    app.unmount("track").expect("mounted");
    settle(&mut app);

    // When
    let refused = handle.invalidate();

    // Then
    assert_eq!(refused, Err(InvalidateError::NotMounted("track".into())));
    assert!(matches!(app.next_after_frame(), Next::Wait));
}

#[test]
fn invalidating_after_a_drawn_frame_wakes_the_loop_the_frame_was_given() {
    // Given: the app kept the platform's waker after a frame, no event
    let mut app = app();
    let handle = app.mount("track", Box::new(Blank)).expect("mounts");
    let platform = Platform::detached();
    app.drawn(&platform);
    let _ = platform.take_redraw_request();

    // When
    let invalidated = handle.invalidate();

    // Then
    assert_eq!(invalidated, Ok(Invalidated::Woke));
    assert!(
        platform.take_redraw_request(),
        "the invalidate woke no frame"
    );
}

#[test]
fn invalidating_before_the_loop_starts_leaves_the_surface_for_the_first_frame() {
    // Given: no event or frame has reached the app
    let mut app = app();
    let handle = app.mount("track", Box::new(Blank)).expect("mounts");
    settle(&mut app);

    // When
    let invalidated = handle.invalidate();

    // Then
    assert_eq!(invalidated, Ok(Invalidated::AtFirstFrame));
    assert!(matches!(app.next_after_frame(), Next::Continue));
}

#[test]
fn invalidating_after_the_app_is_dropped_is_refused() {
    // Given: the app ran, kept a waker, and was dropped
    let mut app = app();
    let handle = app.mount("track", Box::new(Blank)).expect("mounts");
    app.event(Event::PointerLeft, &Platform::detached());

    // When
    drop(app);

    // Then
    assert_eq!(
        handle.invalidate(),
        Err(InvalidateError::LoopClosed("track".into()))
    );
}

#[test]
fn invalidating_after_an_app_that_never_ran_is_dropped_is_refused() {
    // Given
    let mut app = app();
    let handle = app.mount("track", Box::new(Blank)).expect("mounts");

    // When
    drop(app);

    // Then
    assert_eq!(
        handle.invalidate(),
        Err(InvalidateError::LoopClosed("track".into()))
    );
}

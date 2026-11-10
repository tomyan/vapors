//! Animation requests through a `SurfaceHandle` (docs/gpu-surface.md):
//! a request wakes the loop the app was given, or waits for the first
//! frame before the loop starts, and asks for frames; it is refused for
//! a surface no longer mounted and a loop that has ended. Stopping is
//! refused for a surface no longer mounted, and is done once the loop
//! has ended. These tests never reach a GPU.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::{ElementsApp, InvalidateError, Invalidated, SurfaceFrame, SurfaceRenderer};
use vapors_elements::Document;
use vapors_platform::{App, Event, Next, Platform};
use vapors_text::FontSystem;

/// Draws nothing.
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
fn requesting_animation_wakes_the_loop_the_app_was_given() {
    // Given: the app kept the platform's waker from an event
    let mut app = app();
    let handle = app.mount("track", Box::new(Blank)).expect("mounts");
    let platform = Platform::detached();
    app.event(Event::PointerLeft, &platform);
    settle(&mut app);
    let _ = platform.take_redraw_request();

    // When
    let requested = handle.request_animation();

    // Then
    assert_eq!(requested, Ok(Invalidated::Woke));
    assert!(platform.take_redraw_request(), "the request woke no frame");
    assert!(matches!(app.next_after_frame(), Next::Continue));
}

#[test]
fn requesting_animation_before_the_loop_starts_leaves_it_for_the_first_frame() {
    // Given: no event or frame has reached the app
    let mut app = app();
    let handle = app.mount("track", Box::new(Blank)).expect("mounts");
    settle(&mut app);

    // When
    let requested = handle.request_animation();

    // Then
    assert_eq!(requested, Ok(Invalidated::AtFirstFrame));
    assert!(matches!(app.next_after_frame(), Next::Continue));
}

#[test]
fn requesting_or_stopping_animation_of_a_surface_no_longer_mounted_is_refused() {
    // Given
    let mut app = app();
    let handle = app.mount("track", Box::new(Blank)).expect("mounts");
    app.unmount("track").expect("mounted");
    settle(&mut app);

    // When
    let requested = handle.request_animation();
    let stopped = handle.stop_animation();

    // Then
    assert_eq!(requested, Err(InvalidateError::NotMounted("track".into())));
    assert_eq!(stopped, Err(InvalidateError::NotMounted("track".into())));
    assert!(matches!(app.next_after_frame(), Next::Wait));
}

#[test]
fn requesting_animation_after_the_app_is_dropped_is_refused_and_stopping_is_done() {
    // Given: the app ran, kept a waker, and was dropped
    let mut app = app();
    let handle = app.mount("track", Box::new(Blank)).expect("mounts");
    app.event(Event::PointerLeft, &Platform::detached());

    // When
    drop(app);

    // Then
    assert_eq!(
        handle.request_animation(),
        Err(InvalidateError::LoopClosed("track".into()))
    );
    assert_eq!(handle.stop_animation(), Ok(()));
}

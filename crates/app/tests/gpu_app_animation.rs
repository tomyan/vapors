//! GPU tier: animation requests through a `SurfaceHandle`
//! (docs/gpu-surface.md). A requested surface renders every frame and
//! the app asks for the next until it is stopped, then waits; two
//! animating surfaces render once each per frame, and one stopping
//! leaves the other running; a request on a hidden surface asks for no
//! frame and is kept until the surface is shown; a renderer mounted
//! again under an animating name does not animate; a worker thread
//! relays a request through a channel and a wake.

#[path = "support/surface.rs"]
mod support;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use support::{Log, RED, Solid, app_frame, document};
use vapors_app::{ElementsApp, Invalidated, SurfaceHandle};
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_gpu_harness::{Gpu, gpu};
use vapors_platform::{App, Event, Next, Platform};

const PANELS: &str = ".track, .map { width: 16px; height: 16px; } \
    .hidden { visibility: hidden; }";

/// An app showing 16 x 16 surfaces `track` and `map`, its document and
/// its GPU.
fn rig() -> (ElementsApp, Rc<RefCell<Document>>, Gpu) {
    let elements = [(None, "gpu-surface", "track"), (None, "gpu-surface", "map")];
    let doc = Rc::new(RefCell::new(document(PANELS, &elements)));
    (ElementsApp::new(Rc::clone(&doc)), doc, gpu())
}

/// Mounts a red `Solid` as `name`: its handle and its log.
fn mount(app: &mut ElementsApp, name: &str) -> (SurfaceHandle, Rc<RefCell<Log>>) {
    let (solid, log) = Solid::new(RED);
    (app.mount(name, Box::new(solid)).expect("mounts"), log)
}

/// `count` app frames: what each asked for next.
fn frames(app: &mut ElementsApp, gpu: &Gpu, count: usize) -> Vec<Next> {
    (0..count).map(|_| app_frame(app, gpu).0).collect()
}

fn renders(log: &Rc<RefCell<Log>>) -> usize {
    log.borrow().frames.len()
}

#[test]
fn gpu_an_animation_request_renders_every_frame_until_stopped_then_the_app_waits() {
    // Given: both surfaces drawn once, idle
    let (mut app, _doc, gpu) = rig();
    let (track, log) = mount(&mut app, "track");
    mount(&mut app, "map");
    frames(&mut app, &gpu, 1);

    // When
    track.request_animation().expect("mounted");
    let animating = frames(&mut app, &gpu, 3);
    track.stop_animation().expect("mounted");
    let stopped = frames(&mut app, &gpu, 2);

    // Then
    assert_eq!(animating, [Next::Continue; 3]);
    assert_eq!(stopped, [Next::Wait; 2]);
    assert_eq!(renders(&log), 4, "the first frame and the 3 animating");
}

#[test]
fn gpu_two_animating_surfaces_render_once_per_frame_and_one_stopping_leaves_the_other() {
    // Given
    let (mut app, _doc, gpu) = rig();
    let (track, track_log) = mount(&mut app, "track");
    let (map, map_log) = mount(&mut app, "map");
    frames(&mut app, &gpu, 1);
    track.request_animation().expect("mounted");
    map.request_animation().expect("mounted");
    frames(&mut app, &gpu, 2);

    // When
    track.stop_animation().expect("mounted");
    let after = frames(&mut app, &gpu, 2);

    // Then
    assert_eq!(after, [Next::Continue; 2]);
    assert_eq!((renders(&track_log), renders(&map_log)), (3, 5));
}

#[test]
fn gpu_an_animation_requested_on_a_hidden_surface_waits_until_it_is_shown() {
    // Given: `track` hidden
    let (mut app, doc, gpu) = rig();
    let (track, log) = mount(&mut app, "track");
    mount(&mut app, "map");
    set_track_class(&doc, "track hidden");
    frames(&mut app, &gpu, 1);

    // When
    track.request_animation().expect("mounted");
    let hidden = frames(&mut app, &gpu, 2);
    set_track_class(&doc, "track");
    let shown = frames(&mut app, &gpu, 2);

    // Then
    assert_eq!(hidden, [Next::Wait; 2]);
    assert_eq!(shown, [Next::Continue; 2]);
    assert_eq!(renders(&log), 2);
}

fn set_track_class(doc: &Rc<RefCell<Document>>, class: &str) {
    let mut doc = doc.borrow_mut();
    let track = doc.children(doc.body())[0];
    doc.set_attribute(track, "class", class);
}

#[test]
fn gpu_a_renderer_mounted_again_under_an_animating_name_does_not_animate() {
    // Given: `track` animating, then unmounted
    let (mut app, _doc, gpu) = rig();
    let (track, _) = mount(&mut app, "track");
    mount(&mut app, "map");
    track.request_animation().expect("mounted");
    frames(&mut app, &gpu, 2);
    drop(app.unmount("track").expect("mounted"));

    // When
    let (_, log) = mount(&mut app, "track");
    let next = frames(&mut app, &gpu, 3);

    // Then
    assert_eq!(next, [Next::Wait; 3]);
    assert_eq!(renders(&log), 1);
}

#[test]
fn gpu_a_worker_thread_relays_a_request_through_a_channel_and_a_wake() {
    // Given: the app kept the platform's waker from an event
    let (mut app, _doc, gpu) = rig();
    let (track, log) = mount(&mut app, "track");
    mount(&mut app, "map");
    let platform = Platform::detached();
    app.event(Event::PointerLeft, &platform);
    frames(&mut app, &gpu, 1);
    let _ = platform.take_redraw_request();
    let (requests, received) = mpsc::channel();
    let waker = platform.waker();

    // When: the worker cannot hold the handle, so it sends and wakes
    std::thread::spawn(move || {
        requests.send(()).expect("the main thread receives");
        waker.wake().expect("the loop runs");
    })
    .join()
    .expect("the worker finished");
    let woke = platform.take_redraw_request();
    let relayed: Vec<_> = received
        .try_iter()
        .map(|()| track.request_animation())
        .collect();
    let next = frames(&mut app, &gpu, 2);

    // Then
    assert!(woke, "the worker's wake asked for no frame");
    assert_eq!(relayed, [Ok(Invalidated::Woke)]);
    assert_eq!(next, [Next::Continue; 2]);
    assert_eq!(renders(&log), 3);
}

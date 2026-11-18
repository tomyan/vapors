//! The frame clock through the app: a live reader of `frame_time` or
//! `frame_delta` keeps the loop going and sees each frame's time; with
//! no reader the loop idles as before. No GPU: the app is told each
//! frame's `now` through `advance_clock`.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_app::ElementsApp;
use vapors_elements::{Document, frame_delta, frame_time};
use vapors_platform::{Instant, Next};
use vapors_signals::{Owner, Signal};
use vapors_text::FontSystem;

fn app() -> ElementsApp {
    let document = Document::new(FontSystem::bundle_only());
    let mut app = ElementsApp::new(Rc::new(RefCell::new(document)));
    // takes the new document's own change: idle is Wait
    let _ = app.next_after_frame();
    assert!(matches!(app.next_after_frame(), Next::Wait));
    app
}

fn at(start: Instant, ms: u64) -> Instant {
    start + Duration::from_millis(ms)
}

#[test]
fn a_page_without_the_clock_never_continues_for_it() {
    let mut app = app();
    let start = Instant::now();
    app.advance_clock(start);
    app.advance_clock(at(start, 16));
    assert!(matches!(app.next_after_frame(), Next::Wait));
}

#[test]
fn a_reader_continues_and_sees_time_advance_until_it_unmounts() {
    let mut app = app();
    let start = Instant::now();
    let owner = Owner::new();
    let millis = owner.with(|| Signal::derived(|| frame_time().get().as_millis()));
    app.advance_clock(at(start, 500));
    assert_eq!(millis.get_untracked(), 0);
    assert!(matches!(app.next_after_frame(), Next::Continue));
    app.advance_clock(at(start, 516));
    assert_eq!(millis.get_untracked(), 16);
    assert!(matches!(app.next_after_frame(), Next::Continue));
    owner.dispose();
    assert!(matches!(app.next_after_frame(), Next::Wait));
}

#[test]
fn delta_is_zero_on_the_first_frame_and_the_difference_after() {
    let mut app = app();
    let start = Instant::now();
    let owner = Owner::new();
    let delta = owner.with(|| Signal::derived(|| frame_delta().get().as_millis()));
    app.advance_clock(at(start, 40));
    assert_eq!(delta.get_untracked(), 0);
    app.advance_clock(at(start, 57));
    assert_eq!(delta.get_untracked(), 17);
    assert!(matches!(app.next_after_frame(), Next::Continue));
}

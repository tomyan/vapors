//! The booster as the touch stream drives it: a finger cutting a coast
//! and dragging on the same way boosts the fling it releases.

use std::time::Duration;

use vapors_platform::Instant;

use super::TouchScroll;
use crate::fling::Fling;
use crate::fling_curve::FlingCurve;
use crate::scroll_fixture::nested;

const MS: Duration = Duration::from_millis(1);

/// A gesture state with a released fling at `speed` px/s (down the
/// outer list) started at `start` and drawn once a frame later.
fn coasting(
    speed: f32,
    start: Instant,
) -> (
    TouchScroll,
    std::rc::Rc<std::cell::RefCell<vapors_elements::Document>>,
) {
    let (doc, outer, _) = nested();
    let mut touch = TouchScroll::default();
    let velocity = touch.boosted([0.0, speed], start);
    let curve = FlingCurve::new(velocity).expect("moving");
    touch.fling(Fling::new(outer, curve, start));
    touch.advance(&mut doc.borrow_mut(), start + MS * 16);
    (touch, doc)
}

/// A finger down at `t` ms, two drags of `step` px 10 ms apart, lifted;
/// then the release at `release` px/s; the started velocity.
fn cut_and_flick(
    touch: &mut TouchScroll,
    doc: &std::rc::Rc<std::cell::RefCell<vapors_elements::Document>>,
    start: Instant,
    t: u32,
    step: f32,
    release: f32,
) -> [f32; 2] {
    touch.down(1, [10.0, 80.0], start + MS * t);
    touch.pan(&mut doc.borrow_mut(), [0.0, step], start + MS * (t + 10));
    touch.pan(&mut doc.borrow_mut(), [0.0, step], start + MS * (t + 20));
    touch.up(1, start + MS * (t + 25));
    touch.boosted([0.0, release], start + MS * (t + 26))
}

#[test]
fn a_coast_cut_by_a_drag_the_same_way_boosts_the_next_fling() {
    // Given a 1000 px/s coast, 16 ms in
    let start = Instant::now();
    let (mut touch, doc) = coasting(1000.0, start);

    // When a finger cuts it at 30 ms, drags on down at 1000 px/s, and
    // releases at 2000 px/s
    let started = cut_and_flick(&mut touch, &doc, start, 30, 10.0, 2000.0);

    // Then
    assert_eq!(started, [0.0, 3000.0]);
}

#[test]
fn a_drag_against_the_coast_boosts_nothing() {
    // Given
    let start = Instant::now();
    let (mut touch, doc) = coasting(1000.0, start);

    // When the drag goes up, against the coast
    let started = cut_and_flick(&mut touch, &doc, start, 30, -10.0, 2000.0);

    // Then
    assert_eq!(started, [0.0, 2000.0]);
}

#[test]
fn a_coast_that_ended_boosts_nothing() {
    // Given a 1000 px/s coast (555 ms) drawn past its end
    let start = Instant::now();
    let (mut touch, doc) = coasting(1000.0, start);
    touch.advance(&mut doc.borrow_mut(), start + MS * 600);

    // When
    let started = cut_and_flick(&mut touch, &doc, start, 610, 10.0, 2000.0);

    // Then
    assert_eq!(started, [0.0, 2000.0]);
}

#[test]
fn a_coast_stopped_by_the_wheel_boosts_nothing() {
    // Given
    let start = Instant::now();
    let (mut touch, doc) = coasting(1000.0, start);

    // When
    touch.stop_fling();
    let started = cut_and_flick(&mut touch, &doc, start, 30, 10.0, 2000.0);

    // Then
    assert_eq!(started, [0.0, 2000.0]);
}

#[test]
fn a_drag_longer_than_the_window_keeps_it_open_while_it_moves() {
    // Given a 1000 px/s coast cut at 30 ms
    let start = Instant::now();
    let (mut touch, doc) = coasting(1000.0, start);
    touch.down(1, [10.0, 80.0], start + MS * 30);

    // When the drag runs 120 ms, a move every 10 ms at 500 px/s
    for n in 1..=12 {
        touch.pan(
            &mut doc.borrow_mut(),
            [0.0, 5.0],
            start + MS * (30 + 10 * n),
        );
    }
    touch.up(1, start + MS * 155);
    let started = touch.boosted([0.0, 2000.0], start + MS * 156);

    // Then: each move reopened the 50 ms window
    assert_eq!(started, [0.0, 3000.0]);
}

use std::time::Duration;

use vapors_platform::Instant;

use super::Fling;
use crate::fling_curve::FlingCurve;
use crate::scroll_fixture::{nested, offset};

fn curve(speed: f32) -> FlingCurve {
    FlingCurve::new([0.0, speed]).expect("a moving fling")
}

const FRAME: Duration = Duration::from_micros(16_667);

#[test]
fn before_its_start_a_fling_runs_and_moves_nothing() {
    // Given
    let (doc, outer, _) = nested();
    let now = Instant::now();
    let mut fling = Fling::new(outer, curve(500.0), now + FRAME);

    // When
    let running = fling.advance(&mut doc.borrow_mut(), now);

    // Then
    assert!(running);
    assert_eq!(offset(&doc, outer), 0.0);
}

#[test]
fn a_fling_moves_its_container_along_the_curve() {
    // Given a fling of 58.3 px over 333 ms
    let (doc, outer, _) = nested();
    let start = Instant::now();
    let mut fling = Fling::new(outer, curve(500.0), start);

    // When: a first frame, then one a quarter of the way through
    fling.advance(&mut doc.borrow_mut(), start + FRAME);
    let running = fling.advance(&mut doc.borrow_mut(), start + Duration::from_micros(83_318));

    // Then: the spline at 0.25 × 58.3 px
    assert!(running);
    assert!(
        (offset(&doc, outer) - 34.05).abs() < 0.01,
        "{}",
        offset(&doc, outer)
    );
}

#[test]
fn a_delta_of_a_tenth_of_a_pixel_or_less_is_not_sent() {
    // Given a fling one frame in
    let (doc, outer, _) = nested();
    let start = Instant::now();
    let mut fling = Fling::new(outer, curve(500.0), start);
    fling.advance(&mut doc.borrow_mut(), start + FRAME);
    let after_one = offset(&doc, outer);

    // When: 100 µs later, the curve moves about 0.03 px
    let running = fling.advance(
        &mut doc.borrow_mut(),
        start + FRAME + Duration::from_micros(100),
    );

    // Then
    assert!(running);
    assert_eq!(offset(&doc, outer), after_one);
}

#[test]
fn a_fling_ends_at_its_curves_end_without_its_last_delta() {
    // Given a fling of 24.0 px over 229 ms, a frame before its end
    let (doc, outer, _) = nested();
    let start = Instant::now();
    let mut fling = Fling::new(outer, curve(300.0), start);
    fling.advance(&mut doc.borrow_mut(), start + FRAME);
    fling.advance(&mut doc.borrow_mut(), start + Duration::from_millis(200));
    let before_end = offset(&doc, outer);

    // When: a frame past its end
    let running = fling.advance(&mut doc.borrow_mut(), start + Duration::from_millis(230));

    // Then: it ends where the last frame left it, short of 24.0 px
    assert!(!running);
    assert_eq!(offset(&doc, outer), before_end);
    assert!(before_end < 24.0, "{before_end}");
}

#[test]
fn a_fling_stops_at_its_containers_edge() {
    // Given a fling of 1309 px on a list that scrolls 40 px
    let (doc, _, inner) = nested();
    let start = Instant::now();
    let mut fling = Fling::new(inner, curve(3000.0), start);

    // When: frames until it stops
    let frames = (1..100)
        .take_while(|n| fling.advance(&mut doc.borrow_mut(), start + FRAME * *n))
        .count();

    // Then: at the edge within a few frames, then stopped by a frame
    // that moved nothing
    assert_eq!(offset(&doc, inner), 40.0);
    assert!(frames < 5, "{frames} frames");
}

#[test]
fn a_fling_first_drawn_two_frames_late_starts_a_frame_before_that_draw() {
    // Given two flings, one drawn first a frame after its start and one
    // 50 ms after
    let (on_time_doc, on_time_list, _) = nested();
    let (late_doc, late_list, _) = nested();
    let start = Instant::now();
    let mut on_time = Fling::new(on_time_list, curve(500.0), start);
    let mut late = Fling::new(late_list, curve(500.0), start);

    // When
    on_time.advance(&mut on_time_doc.borrow_mut(), start + FRAME);
    late.advance(
        &mut late_doc.borrow_mut(),
        start + Duration::from_millis(50),
    );

    // Then: the late one moved as far as the one on time, not 50 ms' worth
    assert_eq!(
        offset(&late_doc, late_list),
        offset(&on_time_doc, on_time_list)
    );
}

#[test]
fn a_frame_before_the_last_one_that_moved_moves_nothing() {
    // Given a fling that moved at two frames
    let (doc, outer, _) = nested();
    let start = Instant::now();
    let mut fling = Fling::new(outer, curve(500.0), start);
    fling.advance(&mut doc.borrow_mut(), start + FRAME);
    fling.advance(&mut doc.borrow_mut(), start + FRAME * 2);
    let moved = offset(&doc, outer);

    // When: a frame time between the two
    let running = fling.advance(&mut doc.borrow_mut(), start + FRAME * 3 / 2);

    // Then
    assert!(running);
    assert_eq!(offset(&doc, outer), moved);
}

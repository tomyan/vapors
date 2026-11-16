use super::*;

use crate::scroll_fixture::{nested, offset};

fn now() -> vapors_platform::Instant {
    vapors_platform::Instant::now()
}

#[test]
fn a_gesture_keeps_the_container_it_first_moved() {
    // Given: a finger down on the inner list
    let (doc, outer, inner) = nested();
    let mut touch = TouchScroll::default();
    touch.down(1, [10.0, 10.0], now());

    // When: it pans to the inner list's end, then past it
    touch.pan(
        &mut doc.borrow_mut(),
        [0.0, 30.0],
        vapors_platform::Instant::now(),
    );
    touch.pan(
        &mut doc.borrow_mut(),
        [0.0, 30.0],
        vapors_platform::Instant::now(),
    );
    touch.pan(
        &mut doc.borrow_mut(),
        [0.0, 30.0],
        vapors_platform::Instant::now(),
    );

    // Then: the inner list stops at its end; the outer one never moves
    assert_eq!(offset(&doc, inner), 40.0);
    assert_eq!(offset(&doc, outer), 0.0);
}

#[test]
fn the_next_gesture_finds_its_container_again() {
    // Given: a gesture that left the inner list at its end
    let (doc, outer, inner) = nested();
    let mut touch = TouchScroll::default();
    touch.down(1, [10.0, 10.0], now());
    touch.pan(
        &mut doc.borrow_mut(),
        [0.0, 50.0],
        vapors_platform::Instant::now(),
    );
    touch.up(1, now());

    // When: another gesture at the same place
    touch.down(1, [10.0, 10.0], now());
    touch.pan(
        &mut doc.borrow_mut(),
        [0.0, 30.0],
        vapors_platform::Instant::now(),
    );

    // Then: the outer one takes it
    assert_eq!(offset(&doc, inner), 40.0);
    assert_eq!(offset(&doc, outer), 30.0);
}

#[test]
fn a_pan_without_a_finger_down_scrolls_nothing() {
    // Given
    let (doc, outer, inner) = nested();
    let mut touch = TouchScroll::default();

    // When
    let moved = touch.pan(
        &mut doc.borrow_mut(),
        [0.0, 30.0],
        vapors_platform::Instant::now(),
    );

    // Then
    assert!(!moved);
    assert_eq!((offset(&doc, inner), offset(&doc, outer)), (0.0, 0.0));
}

#[test]
fn a_second_finger_lifting_does_not_end_the_gesture() {
    // Given: two fingers down, the gesture scrolling the inner list
    let (doc, _outer, inner) = nested();
    let mut touch = TouchScroll::default();
    touch.down(1, [10.0, 10.0], now());
    touch.down(2, [30.0, 10.0], now());
    touch.pan(
        &mut doc.borrow_mut(),
        [0.0, 10.0],
        vapors_platform::Instant::now(),
    );

    // When: the second finger lifts, and the first pans on
    touch.up(2, now());
    touch.pan(
        &mut doc.borrow_mut(),
        [0.0, 10.0],
        vapors_platform::Instant::now(),
    );

    // Then: still the same gesture, still scrolling
    assert_eq!(offset(&doc, inner), 20.0);

    // When / Then: the last finger lifting ends it; a new gesture finds
    // its container again
    touch.up(1, now());
    touch.down(1, [10.0, 80.0], now());
    touch.pan(
        &mut doc.borrow_mut(),
        [0.0, 10.0],
        vapors_platform::Instant::now(),
    );
    assert_eq!(offset(&doc, inner), 20.0);
}

#[test]
fn a_finger_lifted_twice_or_never_down_ends_nothing() {
    // Given: one finger down and scrolling
    let (doc, _outer, inner) = nested();
    let mut touch = TouchScroll::default();
    touch.down(1, [10.0, 10.0], now());
    touch.pan(
        &mut doc.borrow_mut(),
        [0.0, 10.0],
        vapors_platform::Instant::now(),
    );

    // When: a finger that was never down lifts
    touch.up(7, now());
    touch.pan(
        &mut doc.borrow_mut(),
        [0.0, 10.0],
        vapors_platform::Instant::now(),
    );

    // Then
    assert_eq!(offset(&doc, inner), 20.0);
}

#[test]
fn a_finger_down_stops_a_fling() {
    // Given a fling running on the outer list
    let (doc, outer, _) = nested();
    let start = vapors_platform::Instant::now();
    let curve = crate::fling_curve::FlingCurve::new([0.0, 500.0]).expect("moving");
    let mut touch = TouchScroll::default();
    touch.fling(Fling::new(outer, curve, start));

    // When
    touch.down(1, [10.0, 10.0], now());

    // Then: it no longer runs or moves the list
    let running = touch.advance(
        &mut doc.borrow_mut(),
        start + std::time::Duration::from_millis(50),
    );
    assert!(!running);
    assert!(!touch.flinging());
    assert_eq!(offset(&doc, outer), 0.0);
}

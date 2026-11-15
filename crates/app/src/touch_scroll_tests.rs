use super::*;

use crate::scroll_fixture::{nested, offset};

#[test]
fn a_gesture_keeps_the_container_it_first_moved() {
    // Given: a finger down on the inner list
    let (doc, outer, inner) = nested();
    let mut touch = TouchScroll::default();
    touch.down(1, [10.0, 10.0]);

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
    touch.down(1, [10.0, 10.0]);
    touch.pan(
        &mut doc.borrow_mut(),
        [0.0, 50.0],
        vapors_platform::Instant::now(),
    );
    touch.up(1);

    // When: another gesture at the same place
    touch.down(1, [10.0, 10.0]);
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
fn thumbs_show_with_a_mouse_and_on_touch_from_each_scroll_then_fade() {
    // Given
    let (doc, _outer, _inner) = nested();
    let mut touch = TouchScroll::default();
    let at = vapors_platform::Instant::now();
    let ms = |n: u64| at + std::time::Duration::from_millis(n);
    assert_eq!(
        touch.thumb_opacity(at),
        1.0,
        "a mouse until a finger touches"
    );

    // When / Then: a finger down, not yet scrolling
    touch.down(1, [10.0, 10.0]);
    assert_eq!(touch.thumb_opacity(at), 0.0);

    // When / Then: the gesture scrolls and ends; the thumbs stay, then
    // fade from 300 ms after the scroll over 300 ms
    touch.pan(&mut doc.borrow_mut(), [0.0, 10.0], at);
    touch.up(1);
    assert_eq!(touch.thumb_opacity(ms(10)), 1.0);
    assert!((touch.thumb_opacity(ms(450)) - 0.5).abs() < 1e-3);
    assert_eq!(touch.thumb_opacity(ms(600)), 0.0);

    // When / Then: the mouse again
    touch.mouse();
    assert_eq!(touch.thumb_opacity(ms(600)), 1.0);
}

#[test]
fn a_second_finger_lifting_does_not_end_the_gesture() {
    // Given: two fingers down, the gesture scrolling the inner list
    let (doc, _outer, inner) = nested();
    let mut touch = TouchScroll::default();
    touch.down(1, [10.0, 10.0]);
    touch.down(2, [30.0, 10.0]);
    touch.pan(
        &mut doc.borrow_mut(),
        [0.0, 10.0],
        vapors_platform::Instant::now(),
    );

    // When: the second finger lifts, and the first pans on
    touch.up(2);
    touch.pan(
        &mut doc.borrow_mut(),
        [0.0, 10.0],
        vapors_platform::Instant::now(),
    );

    // Then: still the same gesture, still scrolling
    assert_eq!(offset(&doc, inner), 20.0);

    // When / Then: the last finger lifting ends it; a new gesture finds
    // its container again
    touch.up(1);
    touch.down(1, [10.0, 80.0]);
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
    touch.down(1, [10.0, 10.0]);
    touch.pan(
        &mut doc.borrow_mut(),
        [0.0, 10.0],
        vapors_platform::Instant::now(),
    );

    // When: a finger that was never down lifts
    touch.up(7);
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
    touch.down(1, [10.0, 10.0]);

    // Then: it no longer runs or moves the list
    let running = touch.advance(
        &mut doc.borrow_mut(),
        start + std::time::Duration::from_millis(50),
    );
    assert!(!running);
    assert!(!touch.flinging());
    assert_eq!(offset(&doc, outer), 0.0);
}

#[test]
fn a_fling_shows_the_thumbs_at_each_frame_it_moves_them() {
    // Given a finger that tapped, so the thumbs are hidden
    let (doc, outer, _) = nested();
    let mut touch = TouchScroll::default();
    let at = vapors_platform::Instant::now();
    touch.down(1, [10.0, 10.0]);
    touch.up(1);
    assert_eq!(touch.thumb_opacity(at), 0.0);
    let curve = crate::fling_curve::FlingCurve::new([0.0, 500.0]).expect("moving");
    touch.fling(Fling::new(outer, curve, at));

    // When a frame moves the list
    let frame = at + std::time::Duration::from_millis(16);
    touch.advance(&mut doc.borrow_mut(), frame);

    // Then
    assert_eq!(touch.thumb_opacity(frame), 1.0);
}

#[test]
fn a_fling_stopped_by_the_edge_fades_the_thumbs_and_does_not_hide_them() {
    // Given a fling of 1309 px on a list that scrolls 40 px
    let (doc, _, inner) = nested();
    let mut touch = TouchScroll::default();
    let at = vapors_platform::Instant::now();
    touch.down(1, [10.0, 10.0]);
    touch.up(1);
    let curve = crate::fling_curve::FlingCurve::new([0.0, 3000.0]).expect("moving");
    touch.fling(Fling::new(inner, curve, at));
    let frame = |n: u32| at + std::time::Duration::from_micros(16_667) * n;

    // When: frames until the edge stops it
    let stopped = (1..100)
        .find(|n| !touch.advance(&mut doc.borrow_mut(), frame(*n)))
        .expect("stops");

    // Then: shown at full opacity on the stopping frame, fading after
    assert_eq!(offset(&doc, inner), 40.0);
    assert_eq!(touch.thumb_opacity(frame(stopped)), 1.0);
    let last_moved = frame(stopped - 1);
    let half = last_moved + std::time::Duration::from_millis(450);
    assert!(
        (touch.thumb_opacity(half) - 0.5).abs() < 1e-3,
        "{}",
        touch.thumb_opacity(half)
    );
}

#[test]
fn a_finger_down_does_not_hide_thumbs_still_showing() {
    // Given thumbs shown by a scroll 100 ms ago
    let (doc, _, _) = nested();
    let mut touch = TouchScroll::default();
    let at = vapors_platform::Instant::now();
    touch.down(1, [10.0, 10.0]);
    touch.pan(&mut doc.borrow_mut(), [0.0, 10.0], at);
    touch.up(1);

    // When the next finger goes down
    touch.down(1, [10.0, 10.0]);

    // Then they fade on their own time, as Chrome's do
    assert_eq!(
        touch.thumb_opacity(at + std::time::Duration::from_millis(100)),
        1.0
    );
}

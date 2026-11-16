//! Scroll thumbs on touch: shown by each scroll of their own container,
//! faded after (`thumb_fade.rs`), and fully shown with a mouse.

use super::*;

use crate::scroll_fixture::{nested, offset};

fn now() -> vapors_platform::Instant {
    vapors_platform::Instant::now()
}

#[test]
fn thumbs_show_fully_while_a_mouse_is_in_use() {
    // Given a gesture that scrolled and faded out
    let (doc, _, inner) = nested();
    let mut touch = TouchScroll::default();
    let at = vapors_platform::Instant::now();
    assert_eq!(
        touch.thumb_opacity(inner, at),
        1.0,
        "a mouse until a finger touches"
    );
    touch.down(1, [10.0, 10.0], now());
    touch.pan(&mut doc.borrow_mut(), [0.0, 10.0], at);
    touch.up(1, now());

    // When the mouse moves again
    touch.mouse();

    // Then
    let later = at + std::time::Duration::from_millis(600);
    assert_eq!(touch.thumb_opacity(inner, later), 1.0);
}

#[test]
fn a_finger_down_shows_no_thumbs_until_it_scrolls() {
    // Given
    let (_, _, inner) = nested();
    let mut touch = TouchScroll::default();

    // When
    touch.down(1, [10.0, 10.0], now());

    // Then
    assert_eq!(
        touch.thumb_opacity(inner, vapors_platform::Instant::now()),
        0.0
    );
}

#[test]
fn a_scroll_shows_only_the_scrolled_containers_thumbs() {
    // Given
    let (doc, outer, inner) = nested();
    let mut touch = TouchScroll::default();
    let at = vapors_platform::Instant::now();
    touch.down(1, [10.0, 10.0], now());

    // When the inner list scrolls
    touch.pan(&mut doc.borrow_mut(), [0.0, 10.0], at);

    // Then
    assert_eq!(touch.thumb_opacity(inner, at), 1.0);
    assert_eq!(touch.thumb_opacity(outer, at), 0.0);
}

#[test]
fn thumbs_stay_after_the_lift_then_fade_300_ms_after_the_scroll() {
    // Given a scroll at `at`, then the lift
    let (doc, _, inner) = nested();
    let mut touch = TouchScroll::default();
    let at = vapors_platform::Instant::now();
    let ms = |n: u64| at + std::time::Duration::from_millis(n);
    touch.down(1, [10.0, 10.0], now());
    touch.pan(&mut doc.borrow_mut(), [0.0, 10.0], at);
    touch.up(1, now());

    // Then
    assert_eq!(touch.thumb_opacity(inner, ms(10)), 1.0);
    assert!((touch.thumb_opacity(inner, ms(450)) - 0.5).abs() < 1e-3);
    assert_eq!(touch.thumb_opacity(inner, ms(600)), 0.0);
}

#[test]
fn a_fling_shows_the_thumbs_at_each_frame_it_moves_them() {
    // Given a finger that tapped, so the thumbs are hidden
    let (doc, outer, _) = nested();
    let mut touch = TouchScroll::default();
    let at = vapors_platform::Instant::now();
    touch.down(1, [10.0, 10.0], now());
    touch.up(1, now());
    assert_eq!(touch.thumb_opacity(outer, at), 0.0);
    let curve = crate::fling_curve::FlingCurve::new([0.0, 500.0]).expect("moving");
    touch.fling(Fling::new(outer, curve, at));

    // When a frame moves the list
    let frame = at + std::time::Duration::from_millis(16);
    touch.advance(&mut doc.borrow_mut(), frame);

    // Then
    assert_eq!(touch.thumb_opacity(outer, frame), 1.0);
}

#[test]
fn a_fling_stopped_by_the_edge_fades_the_thumbs_and_does_not_hide_them() {
    // Given a fling of 1309 px on a list that scrolls 40 px
    let (doc, _, inner) = nested();
    let mut touch = TouchScroll::default();
    let at = vapors_platform::Instant::now();
    touch.down(1, [10.0, 10.0], now());
    touch.up(1, now());
    let curve = crate::fling_curve::FlingCurve::new([0.0, 3000.0]).expect("moving");
    touch.fling(Fling::new(inner, curve, at));
    let frame = |n: u32| at + std::time::Duration::from_micros(16_667) * n;

    // When: frames until the edge stops it
    let stopped = (1..100)
        .find(|n| !touch.advance(&mut doc.borrow_mut(), frame(*n)))
        .expect("stops");

    // Then: shown at full opacity on the stopping frame, fading after
    assert_eq!(offset(&doc, inner), 40.0);
    assert_eq!(touch.thumb_opacity(inner, frame(stopped)), 1.0);
    let last_moved = frame(stopped - 1);
    let half = last_moved + std::time::Duration::from_millis(450);
    assert!(
        (touch.thumb_opacity(inner, half) - 0.5).abs() < 1e-3,
        "{}",
        touch.thumb_opacity(inner, half)
    );
}

#[test]
fn a_finger_down_does_not_hide_thumbs_still_showing() {
    // Given thumbs shown by a scroll 100 ms ago
    let (doc, _, inner) = nested();
    let mut touch = TouchScroll::default();
    let at = vapors_platform::Instant::now();
    touch.down(1, [10.0, 10.0], now());
    touch.pan(&mut doc.borrow_mut(), [0.0, 10.0], at);
    touch.up(1, now());

    // When the next finger goes down
    touch.down(1, [10.0, 10.0], now());

    // Then they fade on their own time, as Chrome's do
    assert_eq!(
        touch.thumb_opacity(inner, at + std::time::Duration::from_millis(100)),
        1.0
    );
}

#[test]
fn the_mouse_taking_over_mid_fade_leaves_no_fade_to_draw() {
    // Given a scroll that shows a container's thumbs
    let (doc, _, _) = nested();
    let mut touch = TouchScroll::default();
    let at = vapors_platform::Instant::now();
    touch.down(1, [10.0, 10.0], now());
    touch.pan(&mut doc.borrow_mut(), [0.0, 10.0], at);
    touch.up(1, now());

    // When the mouse moves, and a frame draws the thumbs
    touch.mouse();
    touch.draw_thumbs(&mut doc.borrow_mut(), at);

    // Then no fade asks for frames
    assert_eq!(touch.fades_left(), 0);
}

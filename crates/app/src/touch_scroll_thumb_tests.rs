//! Scroll thumbs on touch: shown by each scroll of their own container,
//! faded after (`thumb_fade.rs`), and fully shown with a mouse.

use super::*;
use crate::thumb_fade::{FADE_DELAY, FADE_DURATION};

use crate::scroll_fixture::{nested, offset};

fn now() -> vapors_platform::Instant {
    vapors_platform::Instant::now()
}

/// Touch in use since a tap whose fade of the mouse's thumbs has ended,
/// and a time after it.
fn after_a_tap() -> (TouchScroll, vapors_platform::Instant) {
    let mut touch = TouchScroll::default();
    let tapped = now();
    touch.down(9, [10.0, 10.0], tapped);
    touch.up(9, tapped);
    (touch, tapped + std::time::Duration::from_secs(1))
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
fn a_first_finger_after_the_mouse_fades_every_thumb_on_the_scroll_timing() {
    // Given thumbs shown fully by the mouse
    let (_, _, inner) = nested();
    let mut touch = TouchScroll::default();
    let at = now();

    // When a finger lands
    touch.down(1, [10.0, 10.0], at);

    // Then they stay 300 ms, are half faded at 450 ms, and gone at 600 ms
    let ms = |n| at + std::time::Duration::from_millis(n);
    assert_eq!(touch.thumb_opacity(inner, ms(0)), 1.0, "not hidden at once");
    assert_eq!(touch.thumb_opacity(inner, ms(299)), 1.0);
    assert!((touch.thumb_opacity(inner, ms(450)) - 0.5).abs() < 1e-3);
    assert_eq!(touch.thumb_opacity(inner, ms(600)), 0.0);
}

#[test]
fn a_later_finger_does_not_show_the_faded_thumbs_again() {
    // Given the first finger's fade ended
    let (_, _, inner) = nested();
    let mut touch = TouchScroll::default();
    let at = now();
    touch.down(1, [10.0, 10.0], at);
    touch.up(1, at);

    // When the next finger lands, the mouse unused since
    let later = at + std::time::Duration::from_millis(700);
    touch.down(1, [10.0, 10.0], later);

    // Then
    assert_eq!(touch.thumb_opacity(inner, later), 0.0);
}

#[test]
fn the_first_fingers_fade_draws_the_shared_thumbs_and_asks_for_frames_until_done() {
    // Given
    let (doc, _, _) = nested();
    let mut touch = TouchScroll::default();
    let at = now();

    // When a finger lands after the mouse
    touch.down(1, [10.0, 10.0], at);

    // Then frames are wanted until the fade ends, and none after
    touch.draw_thumbs(&mut doc.borrow_mut(), at);
    assert_eq!(touch.thumb_phase(at), Phase::Shown(at + FADE_DELAY));
    let done = at + FADE_DELAY + FADE_DURATION;
    touch.draw_thumbs(&mut doc.borrow_mut(), done);
    assert_eq!(touch.thumb_phase(done), Phase::Hidden);
}

#[test]
fn a_scroll_shows_only_the_scrolled_containers_thumbs() {
    // Given
    let (doc, outer, inner) = nested();
    let (mut touch, at) = after_a_tap();
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
    let (mut touch, at) = after_a_tap();
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
    // Given a finger that tapped, its fade over, so the thumbs are hidden
    let (doc, outer, _) = nested();
    let (mut touch, at) = after_a_tap();
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
    let (mut touch, at) = after_a_tap();
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

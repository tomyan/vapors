//! A touch gesture's fling: the container the gesture scrolled coasts
//! at the lifted finger's velocity, within Chromium's bounds.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_dom::NodeId;
use vapors_elements::Document;
use vapors_platform::{Event, Instant, Platform, TouchPhase};

use super::DocumentInput;
use crate::scroll_fixture::{nested, offset};

fn touch(phase: TouchPhase, y: f32) -> Event {
    Event::Touch {
        id: 1,
        phase,
        x: 10.0,
        y,
    }
}

/// A finger down at `y` on the nested lists (scale 1) dragging 25 px
/// up, lifted at `at`, then the platform's fling at `vy` (the finger's
/// way).
fn flick(y: f32, vy: f32, at: Instant) -> (Rc<RefCell<Document>>, NodeId, DocumentInput) {
    let (doc, outer, _) = nested();
    let (mut input, platform) = (DocumentInput::new(), Platform::detached());
    let events = [
        touch(TouchPhase::Started, y),
        touch(TouchPhase::Moved, y - 25.0),
        Event::Pan { dx: 0.0, dy: -25.0 },
        touch(TouchPhase::Ended, y - 25.0),
        Event::Fling { vx: 0.0, vy, at },
    ];
    for event in &events {
        input.give(&doc, event, &platform).expect("placed");
    }
    (doc, outer, input)
}

#[test]
fn a_flicked_list_coasts_on_after_the_finger_lifts() {
    // Given the outer list, dragged 25 px and flicked up at 600 px/s
    let lift = Instant::now();
    let (doc, outer, mut input) = flick(80.0, -600.0, lift);
    assert!(input.flinging());

    // When
    input.advance_fling(&mut doc.borrow_mut(), lift + Duration::from_millis(16));
    input.advance_fling(&mut doc.borrow_mut(), lift + Duration::from_millis(100));

    // Then
    assert!(offset(&doc, outer) > 30.0, "{}", offset(&doc, outer));
}

#[test]
fn a_finger_lifted_at_50_px_a_second_or_slower_does_not_fling() {
    // Given, When
    let (_, _, input) = flick(80.0, -50.0, Instant::now());

    // Then
    assert!(!input.flinging());
}

#[test]
fn a_fling_after_a_gesture_that_scrolled_nothing_starts_nothing() {
    // Given a flick, then a tap, which stops its fling
    let (doc, _, mut input) = flick(80.0, -600.0, Instant::now());
    let platform = Platform::detached();
    for event in [
        touch(TouchPhase::Started, 80.0),
        touch(TouchPhase::Ended, 80.0),
    ] {
        input.give(&doc, &event, &platform).expect("placed");
    }

    // When
    input
        .give(
            &doc,
            &Event::Fling {
                vx: 0.0,
                vy: -600.0,
                at: Instant::now(),
            },
            &platform,
        )
        .expect("placed");

    // Then
    assert!(!input.flinging());
}

#[test]
fn a_fling_faster_than_8000_px_a_second_coasts_as_8000_does() {
    // Given three flicks lifted at one instant, at, over and under the
    // maximum, each drawn 3 ms after the lift
    let lift = Instant::now();
    let moved = |vy: f32| {
        let (doc, outer, mut input) = flick(80.0, vy, lift);
        let start = offset(&doc, outer);
        input.advance_fling(&mut doc.borrow_mut(), lift + Duration::from_millis(3));
        offset(&doc, outer) - start
    };

    // When
    let (max, over, under) = (moved(-8000.0), moved(-50_000.0), moved(-7000.0));

    // Then: on one clock the clamped fling moves exactly as the maximum
    // does; 1000 px/s slower moves at least half of 1000 px/s × 3 ms less
    assert_eq!(over, max);
    assert!(max - under > 1.5, "{max} vs {under}");
}

#[test]
fn a_second_fling_for_one_gesture_starts_nothing() {
    // Given a flick whose fling the wheel stopped
    let (doc, _, mut input) = flick(80.0, -600.0, Instant::now());
    let platform = Platform::detached();
    input
        .give(&doc, &Event::Scroll { dx: 0.0, dy: 1.0 }, &platform)
        .expect("placed");
    assert!(!input.flinging());

    // When another fling arrives with no gesture between
    let again = Event::Fling {
        vx: 0.0,
        vy: -600.0,
        at: Instant::now(),
    };
    input.give(&doc, &again, &platform).expect("placed");

    // Then
    assert!(!input.flinging());
}

#[test]
fn a_touch_fling_is_boosted_from_and_a_programmatic_one_is_not() {
    // Given a flick, which the next touch fling could boost from
    let (doc, outer, mut input) = flick(80.0, -600.0, Instant::now());
    assert_eq!(input.touch.boost_from(), [0.0, 600.0]);

    // When the app flings the list itself
    input
        .fling(&doc.borrow(), outer, [0.0, 900.0], Instant::now())
        .expect("starts");

    // Then
    assert_eq!(input.touch.boost_from(), [0.0, 0.0]);
}

#[test]
fn a_boosted_release_starts_above_the_maximum_unclamped() {
    // Given a 3000 px/s flick lifted 100 ms ago, drawn once since (about
    // 48 px, inside the outer list's 100)
    let lift = Instant::now()
        .checked_sub(Duration::from_millis(100))
        .expect("a clock past 100 ms");
    let (doc, _, mut input) = flick(80.0, -3000.0, lift);
    input.advance_fling(&mut doc.borrow_mut(), lift + Duration::from_millis(16));
    let platform = Platform::detached();

    // When a finger cuts the coast, drags on, and flicks at the maximum
    for event in [
        touch(TouchPhase::Started, 80.0),
        touch(TouchPhase::Moved, 55.0),
        Event::Pan { dx: 0.0, dy: -25.0 },
        touch(TouchPhase::Ended, 55.0),
        Event::Fling {
            vx: 0.0,
            vy: -8000.0,
            at: Instant::now(),
        },
    ] {
        input.give(&doc, &event, &platform).expect("placed");
    }

    // Then: the boost adds the first start after the clamp, and the
    // fling runs at the sum, unclamped (the curve's start is within 2%)
    assert_eq!(input.touch.boost_from(), [0.0, 11_000.0]);
    let speed = input.touch.fling_starting_speed().expect("flinging");
    assert!(speed > 10_500.0, "{speed}");
}

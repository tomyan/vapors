use std::time::Duration;

use vapors_platform::{Event, Instant, Platform, TouchPhase};

use super::{DocumentInput, FlingError, MAX_FLING_SPEED};
use crate::scroll_fixture::{nested, offset};

#[test]
fn a_fling_moves_its_container_at_each_advance_until_it_ends() {
    // Given
    let (doc, outer, _) = nested();
    let mut input = DocumentInput::new();
    let start = Instant::now();
    input
        .fling(&doc.borrow(), outer, [0.0, 500.0], start)
        .expect("starts");

    // When
    input.advance_fling(&mut doc.borrow_mut(), start + Duration::from_millis(16));
    let running = input.advance_fling(&mut doc.borrow_mut(), start + Duration::from_millis(100));
    let ended = !input.advance_fling(&mut doc.borrow_mut(), start + Duration::from_secs(1));

    // Then
    assert!(running && ended);
    assert!(offset(&doc, outer) > 30.0, "{}", offset(&doc, outer));
    assert!(!input.flinging());
}

#[test]
fn a_fling_at_an_unreadable_velocity_is_refused() {
    // Given
    let (doc, outer, _) = nested();
    let mut input = DocumentInput::new();

    // When
    let nan = input.fling(&doc.borrow(), outer, [f32::NAN, 0.0], Instant::now());

    // Then
    assert!(
        matches!(nan, Err(FlingError::UnreadableVelocity(_))),
        "{nan:?}"
    );
    assert!(!input.flinging());
}

#[test]
fn a_fling_of_an_element_that_does_not_scroll_is_refused() {
    // Given
    let (doc, _, _) = nested();
    let body = doc.borrow().body();
    let mut input = DocumentInput::new();

    // When
    let started = input.fling(&doc.borrow(), body, [0.0, 500.0], Instant::now());

    // Then
    assert_eq!(started, Err(FlingError::NotAScroller(body)));
    assert!(!input.flinging());
}

#[test]
fn a_zero_velocity_starts_no_fling() {
    // Given
    let (doc, outer, _) = nested();
    let mut input = DocumentInput::new();

    // When
    let started = input.fling(&doc.borrow(), outer, [0.0, 0.0], Instant::now());

    // Then
    assert_eq!(started, Ok(()));
    assert!(!input.flinging());
}

#[test]
fn a_fling_while_a_finger_is_down_is_refused() {
    // Given a finger down on the outer list
    let (doc, outer, _) = nested();
    let (mut input, platform) = (DocumentInput::new(), Platform::detached());
    let down = Event::Touch {
        id: 1,
        phase: TouchPhase::Started,
        x: 10.0,
        y: 80.0,
    };
    input.give(&doc, &down, &platform).expect("placed");

    // When
    let started = input.fling(&doc.borrow(), outer, [0.0, 500.0], Instant::now());

    // Then
    assert_eq!(started, Err(FlingError::FingerDown));
    assert!(!input.flinging());
}

#[test]
fn a_fling_at_any_speed_runs_at_most_the_maximum_and_ends() {
    // Given flings at the maximum and far beyond it, which the curve
    // could not carry (1e30 px/s)
    let run = |speed: f32| {
        let (doc, outer, _) = nested();
        let mut input = DocumentInput::new();
        let start = Instant::now();
        input
            .fling(&doc.borrow(), outer, [0.0, speed], start)
            .expect("starts");
        let first = start + Duration::from_millis(2);
        input.advance_fling(&mut doc.borrow_mut(), first);
        let after_one = offset(&doc, outer);
        let frames = (2..1000)
            .take_while(|n| {
                input.advance_fling(
                    &mut doc.borrow_mut(),
                    start + Duration::from_millis(16) * *n,
                )
            })
            .count();
        (after_one, frames)
    };

    // When
    let (at_max, over) = (run(MAX_FLING_SPEED), run(1e30));

    // Then: the same first frame (2 ms in, inside the 100 px range), and
    // both stop at the list's end
    assert_eq!(over.0, at_max.0);
    assert!(over.1 < 10 && at_max.1 < 10, "{over:?} {at_max:?}");
}

#[test]
fn the_wheel_or_a_press_stops_a_fling() {
    for event in [
        Event::Scroll { dx: 0.0, dy: 10.0 },
        Event::PrimaryButton {
            pressed: true,
            clicks: std::num::NonZeroU32::MIN,
            modifiers: vapors_platform::Modifiers::default(),
        },
    ] {
        // Given a fling running on the outer list
        let (doc, outer, _) = nested();
        let (mut input, platform) = (DocumentInput::new(), Platform::detached());
        input
            .fling(&doc.borrow(), outer, [0.0, 500.0], Instant::now())
            .expect("starts");

        // When
        input.give(&doc, &event, &platform).expect("placed");

        // Then
        assert!(!input.flinging(), "{event:?}");
    }
}

#[test]
fn losing_the_windows_focus_does_not_stop_a_fling() {
    // Given
    let (doc, outer, _) = nested();
    let (mut input, platform) = (DocumentInput::new(), Platform::detached());
    input
        .fling(&doc.borrow(), outer, [0.0, 500.0], Instant::now())
        .expect("starts");

    // When
    input
        .give(&doc, &Event::WindowFocus { focused: false }, &platform)
        .expect("placed");

    // Then
    assert!(input.flinging());
}

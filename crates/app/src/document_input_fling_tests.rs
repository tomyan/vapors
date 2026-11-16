use std::time::Duration;

use vapors_platform::{Event, Instant, Platform, TouchPhase};

use super::{CURVE_SPEED_LIMIT, DocumentInput, FlingError, MAX_FLING_SPEED};
use crate::scroll_fixture::{nested, offset};
use crate::thumb_fade::Phase;

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
fn a_fling_the_curve_cannot_carry_is_refused_and_one_it_can_is_not_clamped() {
    // Given
    let (doc, outer, _) = nested();
    let mut input = DocumentInput::new();
    let now = Instant::now();

    // When: past the curve's limit, then a boosted chain's speed above
    // the release maximum
    let absurd = input.fling(&doc.borrow(), outer, [0.0, 1e30], now);
    let chained = input.fling(&doc.borrow(), outer, [0.0, 3.0 * MAX_FLING_SPEED], now);

    // Then
    assert_eq!(absurd, Err(FlingError::TooFast([0.0, 1e30])));
    assert_eq!(chained, Ok(()));
    const { assert!(CURVE_SPEED_LIMIT > 3.0 * MAX_FLING_SPEED) };
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

#[test]
fn a_first_touch_on_nothing_asks_for_frames_until_the_thumbs_have_faded() {
    // Given the mouse's thumbs, and a point with nothing painted under it
    let (doc, _, _) = nested();
    let (mut input, platform) = (DocumentInput::new(), Platform::detached());
    let before = Instant::now();
    let down = Event::Touch {
        id: 1,
        phase: TouchPhase::Started,
        x: 150.0,
        y: 150.0,
    };

    assert_eq!(doc.borrow().hit([150.0, 150.0]), None);
    doc.borrow_mut().take_changed();

    // When a finger lands there, handed to the document as ElementsApp
    // hands it every event (`give` routes it to the host)
    let handled = input.handle(&doc, &down, &platform).expect("placed");

    // Then it asks for a frame, and each frame for the next until faded
    assert!(handled.changed, "the fade's first frame is asked for");
    let phase_at = |input: &mut DocumentInput, ms| {
        input.advance_fling(&mut doc.borrow_mut(), before + Duration::from_millis(ms));
        input.thumb_phase()
    };
    assert!(matches!(phase_at(&mut input, 0), Phase::Shown(_)));
    assert_eq!(phase_at(&mut input, 450), Phase::Fading);
    assert_eq!(phase_at(&mut input, 1600), Phase::Hidden);
}

use std::time::Duration;

use vapors_platform::Instant;

use super::{DocumentInput, FlingError};
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
    let running = input.advance_fling(&mut doc.borrow_mut(), start + Duration::from_millis(100));
    let ended = !input.advance_fling(&mut doc.borrow_mut(), start + Duration::from_secs(1));

    // Then
    assert!(running && ended);
    assert!(offset(&doc, outer) > 30.0, "{}", offset(&doc, outer));
    assert!(!input.flinging());
}

#[test]
fn a_fling_at_an_unreadable_velocity_or_of_no_scroller_is_refused() {
    // Given
    let (doc, outer, _) = nested();
    let body = doc.borrow().body();
    let mut input = DocumentInput::new();
    let now = Instant::now();

    // When
    let nan = input.fling(&doc.borrow(), outer, [f32::NAN, 0.0], now);
    let not_a_scroller = input.fling(&doc.borrow(), body, [0.0, 500.0], now);

    // Then
    assert!(
        matches!(nan, Err(FlingError::UnreadableVelocity(_))),
        "{nan:?}"
    );
    assert_eq!(not_a_scroller, Err(FlingError::NotAScroller(body)));
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

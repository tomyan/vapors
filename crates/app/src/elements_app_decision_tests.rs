use std::time::Duration;

use vapors_platform::{Instant, Next};

use super::FrameDecision;

const IDLE: FrameDecision = FrameDecision {
    document_changed: false,
    surface_invalidated: false,
    surfaces_want: false,
};

#[test]
fn an_idle_frame_with_no_pending_scope_waits_for_events_alone() {
    assert_eq!(IDLE.next(None), Next::Wait);
}

#[test]
fn an_idle_frame_with_a_pending_scope_waits_until_its_deadline() {
    // Given
    let deadline = Instant::now() + Duration::from_millis(1000);

    // When
    let next = IDLE.next(Some(deadline));

    // Then
    assert_eq!(next, Next::WaitUntil(deadline));
}

#[test]
fn a_frame_wanting_another_draws_at_once_whatever_the_deadline() {
    // Given
    let changed = FrameDecision {
        document_changed: true,
        ..IDLE
    };
    let deadline = Instant::now() + Duration::from_millis(1000);

    // When / Then
    assert_eq!(changed.next(Some(deadline)), Next::Continue);
    assert_eq!(changed.next(None), Next::Continue);
}

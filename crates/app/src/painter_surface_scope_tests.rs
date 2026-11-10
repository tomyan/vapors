use std::time::Duration;

use vapors_platform::{Instant, Next};

use super::fakes::{Late, counting, woken};
use super::{DEADLINE, LoopWaker, Scopes, Turn};
use crate::FrameDecision;

/// Scopes holding one `Late` scope begun at `start`, checked once.
fn waiting_on(late: &Late, waker: &LoopWaker, start: Instant) -> Scopes {
    let mut scopes = Scopes::default();
    scopes.wait_for(Box::pin(late.clone()), Turn { waker, now: start });
    assert!(scopes.check(start).is_ok());
    scopes
}

#[test]
fn a_pending_scope_gives_its_deadline_for_the_loop_to_wait_until() {
    // Given
    let (_, waker) = counting();
    let start = Instant::now();

    // When
    let scopes = waiting_on(&Late::default(), &waker, start);

    // Then
    assert_eq!(scopes.deadline(), Some(start + DEADLINE));
}

#[test]
fn a_clean_late_result_wakes_nothing_and_the_next_check_clears_it() {
    // Given
    let (count, waker) = counting();
    let start = Instant::now();
    let late = Late::default();
    let mut scopes = waiting_on(&late, &waker, start);

    // When
    late.resolve(None);
    let checked = scopes.check(start + Duration::from_millis(1));

    // Then
    assert_eq!(woken(&count), 0);
    assert!(checked.is_ok());
    assert_eq!(scopes.deadline(), None);
}

#[test]
fn an_error_in_a_late_result_refuses_the_next_check() {
    // Given
    let (_, waker) = counting();
    let start = Instant::now();
    let late = Late::default();
    let mut scopes = waiting_on(&late, &waker, start);

    // When
    late.resolve(Some("bad bind group"));
    let checked = scopes.check(start + Duration::from_millis(1));

    // Then
    let refusal = format!("{:?}", checked.expect_err("the late error refuses"));
    assert!(refusal.contains("bad bind group"), "{refusal}");
}

#[test]
fn a_scope_unresolved_at_the_deadline_refuses_with_no_wake_before() {
    // Given
    let (count, waker) = counting();
    let start = Instant::now();
    let mut scopes = waiting_on(&Late::default(), &waker, start);
    let just_before = start + DEADLINE.saturating_sub(Duration::from_millis(1));

    // When
    let before = scopes.check(just_before);
    let at = scopes.check(start + DEADLINE);
    let after = scopes.check(start + DEADLINE * 2);

    // Then
    assert_eq!(woken(&count), 0);
    assert!(before.is_ok());
    let at = format!("{:?}", at.expect_err("refused at the deadline"));
    assert!(at.contains("within 1000 ms"), "{at}");
    assert!(after.is_err());
    assert_eq!(scopes.deadline(), None);
}

/// What an idle app's loop does after a frame of `scopes`.
fn idle_next(scopes: &Scopes) -> Next {
    FrameDecision::default().next(scopes.deadline())
}

#[test]
fn a_frame_after_a_scope_resolves_waits_for_events_alone() {
    // Given
    let (_, waker) = counting();
    let start = Instant::now();
    let late = Late::default();
    let mut scopes = waiting_on(&late, &waker, start);
    let asked = idle_next(&scopes);

    // When
    late.resolve(None);
    let woken_frame = scopes.check(start + Duration::from_millis(1));

    // Then
    assert_eq!(asked, Next::WaitUntil(start + DEADLINE));
    assert!(woken_frame.is_ok());
    assert_eq!(idle_next(&scopes), Next::Wait);
}

#[test]
fn a_scope_never_resolving_gets_one_frame_at_its_deadline_which_refuses() {
    // Given
    let (count, waker) = counting();
    let start = Instant::now();
    let mut scopes = waiting_on(&Late::default(), &waker, start);
    let asked = idle_next(&scopes);

    // When
    let deadline_frame = scopes.check(start + DEADLINE);

    // Then
    assert_eq!(asked, Next::WaitUntil(start + DEADLINE));
    assert_eq!(woken(&count), 0);
    assert!(deadline_frame.is_err());
    assert_eq!(idle_next(&scopes), Next::Wait);
}

#[test]
fn a_late_error_seen_first_after_the_deadline_is_refused_as_itself() {
    // Given
    let (_, waker) = counting();
    let start = Instant::now();
    let late = Late::default();
    let mut scopes = waiting_on(&late, &waker, start);

    // When
    late.resolve(Some("bad bind group"));
    let checked = scopes.check(start + DEADLINE * 2);

    // Then
    let refusal = format!("{:?}", checked.expect_err("the late error refuses"));
    assert!(refusal.contains("bad bind group"), "{refusal}");
    assert!(!refusal.contains("within"), "{refusal}");
}

#[test]
fn settling_past_the_deadline_gives_up_with_no_check_and_the_next_check_refuses() {
    // Given
    let (_, waker) = counting();
    let start = Instant::now();
    let mut scopes = waiting_on(&Late::default(), &waker, start);

    // When
    scopes.settle(start + DEADLINE);

    // Then
    assert_eq!(idle_next(&scopes), Next::Wait);
    assert!(scopes.check(start + DEADLINE).is_err());
}

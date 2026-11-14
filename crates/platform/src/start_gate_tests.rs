use std::time::Duration;

use super::{SIZE_DEADLINE, StartGate, Step};
use crate::Instant;

fn ready_at(now: Instant) -> StartGate {
    let mut gate = StartGate::default();
    gate.gpu_ready(now);
    gate
}

#[test]
fn a_sized_window_starts_when_the_gpu_is_ready() {
    // Given
    let now = Instant::now();
    let mut gate = ready_at(now);

    // When
    let step = gate.step((800, 600), now);

    // Then
    assert_eq!(step, Step::Start);
}

#[test]
fn a_sized_window_does_not_start_before_the_gpu_is_ready() {
    // Given
    let mut gate = StartGate::default();

    // When
    let step = gate.step((800, 600), Instant::now());

    // Then
    assert_eq!(step, Step::Wait);
}

#[test]
fn a_window_with_no_size_waits_then_starts_at_its_first_size() {
    // Given
    let now = Instant::now();
    let mut gate = ready_at(now);
    let waited = gate.step((0, 0), now);

    // When
    let later = gate.step((800, 600), now + Duration::from_millis(500));

    // Then
    assert_eq!(waited, Step::WaitUntil(now + SIZE_DEADLINE));
    assert_eq!(later, Step::Start);
}

#[test]
fn a_window_with_no_size_at_the_deadline_is_refused_naming_the_fix() {
    // Given
    let now = Instant::now();
    let mut gate = ready_at(now);
    gate.step((0, 600), now);

    // When
    let step = gate.step((0, 600), now + SIZE_DEADLINE);

    // Then
    let Step::Refuse(message) = step else {
        panic!("not refused: {step:?}");
    };
    assert!(
        message.contains("0x600") && message.contains("fix:"),
        "{message}"
    );
}

#[test]
fn a_started_gate_starts_nothing_more() {
    // Given
    let now = Instant::now();
    let mut gate = ready_at(now);
    gate.step((800, 600), now);

    // When
    let again = gate.step((1024, 768), now);

    // Then
    assert_eq!(again, Step::Started);
}

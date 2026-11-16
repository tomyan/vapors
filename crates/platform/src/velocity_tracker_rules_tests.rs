//! The tracker's history rules (20 samples, 100 ms, the 40 ms and
//! 80 ms stops), an exact quadratic, and what it answers when it cannot
//! measure (`velocity_tracker.rs`).

use std::time::Duration;

use super::tests::{NO_LIMIT, moves, near, speed};
use super::{Unresolvable, Velocity, VelocityTracker};

#[test]
fn an_up_within_80_ms_keeps_the_velocity() {
    // Given
    let mut tracker = VelocityTracker::default();
    tracker.down([0.0, 0.0], Duration::ZERO);
    moves(
        &mut tracker,
        [0.0, 0.0],
        [0.0, 1000.0],
        Duration::ZERO,
        Duration::from_millis(80),
        10,
    );

    // When: 79 ms after the last move (at 72 ms)
    tracker.up(Duration::from_millis(151));

    // Then
    near(speed(&tracker, NO_LIMIT)[1], 1000.0, 1.0);
}

#[test]
fn moves_40_ms_apart_start_the_history_again() {
    // Given a fast stretch, then a 40 ms pause
    let mut tracker = VelocityTracker::default();
    tracker.down([0.0, 0.0], Duration::ZERO);
    moves(
        &mut tracker,
        [0.0, 0.0],
        [0.0, 3000.0],
        Duration::ZERO,
        Duration::from_millis(30),
        6,
    );

    // When: two moves at 200 px/s after it
    tracker.moved([0.0, 100.0], Duration::from_millis(65));
    tracker.moved([0.0, 101.0], Duration::from_millis(70));

    // Then: only those two count
    near(speed(&tracker, NO_LIMIT)[1], 200.0, 0.5);
}

#[test]
fn samples_older_than_100_ms_before_the_newest_do_not_count() {
    // Given a stretch at 3000 px/s, then 101 ms of 500 px/s, 20 ms apart
    let mut tracker = VelocityTracker::default();
    tracker.down([0.0, 0.0], Duration::ZERO);
    moves(
        &mut tracker,
        [0.0, 0.0],
        [0.0, 3000.0],
        Duration::ZERO,
        Duration::from_millis(60),
        3,
    );

    // When
    moves(
        &mut tracker,
        [0.0, 120.0],
        [0.0, 500.0],
        Duration::from_millis(60),
        Duration::from_millis(120),
        6,
    );

    // Then: the newest at 160 ms, the 3000 px/s samples at 40 ms and before
    near(speed(&tracker, NO_LIMIT)[1], 500.0, 0.5);
}

#[test]
fn at_most_20_samples_count() {
    // Given a jump, then 20 samples at 1000 px/s 1 ms apart (the ring
    // has wrapped, so all 20 slots hold samples)
    let mut tracker = VelocityTracker::default();
    tracker.down([0.0, 0.0], Duration::ZERO);
    tracker.moved([0.0, 500.0], Duration::from_millis(1));

    // When
    moves(
        &mut tracker,
        [0.0, 0.0],
        [0.0, 1000.0],
        Duration::from_millis(2),
        Duration::from_millis(20),
        20,
    );

    // Then: the jump is the 21st newest, outside the history
    near(speed(&tracker, NO_LIMIT)[1], 1000.0, 0.5);
}

#[test]
fn a_quadratic_path_is_measured_at_its_newest_sample() {
    // Given y = 2000 t + 10000 t^2 sampled every 8 ms to 64 ms: LSQ2 fits
    // it exactly, and its slope at the newest sample is 2000 + 20000 t
    let mut tracker = VelocityTracker::default();
    let y = |ms: u32| {
        let t = f64::from(ms) / 1000.0;
        #[allow(clippy::cast_possible_truncation, reason = "a few hundred pixels")]
        let value = (2000.0 * t + 10_000.0 * t * t) as f32;
        value
    };
    tracker.down([0.0, 0.0], Duration::ZERO);
    for ms in (8..=64).step_by(8) {
        tracker.moved([0.0, y(ms)], Duration::from_millis(u64::from(ms)));
    }

    // Then
    near(speed(&tracker, NO_LIMIT)[1], 3280.0, 2.0);
}

#[test]
fn a_single_sample_has_no_velocity() {
    let mut tracker = VelocityTracker::default();
    tracker.down([3.0, 4.0], Duration::ZERO);
    assert_eq!(tracker.velocity(NO_LIMIT), Velocity::Stopped);
}

#[test]
fn moves_handled_together_after_a_stall_are_unresolvable() {
    // Given moves every 8 ms, then a 45 ms stall, then three moves
    // handled at one time
    let mut tracker = VelocityTracker::default();
    tracker.down([0.0, 0.0], Duration::ZERO);
    moves(
        &mut tracker,
        [0.0, 0.0],
        [0.0, 1000.0],
        Duration::from_millis(8),
        Duration::from_millis(40),
        5,
    );
    let stalled = Duration::from_millis(85);
    for y in [80.0, 88.0, 96.0] {
        tracker.moved([0.0, y], stalled);
    }

    // When the finger lifts the next frame
    tracker.up(Duration::from_millis(100));

    // Then
    assert_eq!(
        tracker.velocity(NO_LIMIT),
        Velocity::Unresolvable(Unresolvable::Stalled)
    );
}

#[test]
fn samples_at_too_few_times_to_fit_are_unresolvable() {
    // Given a down, then two moves handled at one time (coalesced on the
    // web): ages {-16 ms, 0, 0} leave the t^2 column a multiple of t
    let mut tracker = VelocityTracker::default();
    tracker.down([0.0, 0.0], Duration::ZERO);
    tracker.moved([0.0, 10.0], Duration::from_millis(16));
    tracker.moved([0.0, 14.0], Duration::from_millis(16));

    // Then
    assert_eq!(
        tracker.velocity(NO_LIMIT),
        Velocity::Unresolvable(Unresolvable::Degenerate)
    );
}

#[test]
fn a_finger_held_still_then_moving_again_is_measured_afresh() {
    // Given a stop of 45 ms, then moves 8 ms apart: a stopped finger, not
    // a stall, so the history restarts and measures
    let mut tracker = VelocityTracker::default();
    tracker.down([0.0, 0.0], Duration::ZERO);
    tracker.moved([0.0, 10.0], Duration::from_millis(8));
    moves(
        &mut tracker,
        [0.0, 10.0],
        [0.0, 500.0],
        Duration::from_millis(53),
        Duration::from_millis(24),
        3,
    );

    // Then
    near(speed(&tracker, NO_LIMIT)[1], 500.0, 0.5);
}

//! Chromium's own tests of LSQ2
//! (`ui/events/velocity_tracker/velocity_tracker_unittest.cc`: `Basic`,
//! `MaxVelocity`, `VaryingVelocity`, `DelayedActionUp`) on the same
//! synthetic samples. The history's rules are in
//! `velocity_tracker_rules_tests.rs`.

use std::time::Duration;

use super::{Velocity, VelocityTracker};

pub(super) const NO_LIMIT: f32 = 20_000.0;

fn at(v: [f32; 2], t: Duration) -> [f32; 2] {
    let s = t.as_secs_f32();
    [v[0] * s, v[1] * s]
}

/// `ApplyMovement`: `samples` moves at velocity `v` over `span` from
/// `start`, from the position at `start` (`from`).
pub(super) fn moves(
    tracker: &mut VelocityTracker,
    from: [f32; 2],
    v: [f32; 2],
    start: Duration,
    span: Duration,
    samples: u32,
) {
    let dt = span / samples;
    for i in 0..samples {
        let p = at(v, dt * i);
        tracker.moved([from[0] + p[0], from[1] + p[1]], start + dt * i);
    }
}

/// The tracker's velocity, which must be a measured one.
pub(super) fn speed(tracker: &VelocityTracker, max: f32) -> [f32; 2] {
    match tracker.velocity(max) {
        Velocity::Moving(velocity) => velocity,
        other => panic!("no velocity measured: {other:?}"),
    }
}

pub(super) fn near(got: f32, expected: f32, within: f32) {
    assert!(
        (got - expected).abs() <= within,
        "{got}, not {expected} (within {within})"
    );
}

#[test]
fn a_constant_velocity_is_measured_as_itself() {
    // Given Chromium's Basic: 60 samples over a second at (0, 500)
    let mut tracker = VelocityTracker::default();
    assert_eq!(tracker.velocity(NO_LIMIT), Velocity::Stopped);
    tracker.down([0.0, 0.0], Duration::ZERO);
    moves(
        &mut tracker,
        [0.0, 0.0],
        [0.0, 500.0],
        Duration::ZERO,
        Duration::from_secs(1),
        60,
    );

    // When
    tracker.up(Duration::from_secs(1));

    // Then
    let [x, y] = speed(&tracker, NO_LIMIT);
    near(x, 0.0, 0.01);
    near(y, 500.0, 5.0);
}

#[test]
fn each_axis_is_clamped_to_the_maximum() {
    // Given Chromium's MaxVelocity: 3 samples over 20 ms at (-50000, 50000)
    let mut tracker = VelocityTracker::default();
    let v = [-50_000.0, 50_000.0];
    tracker.down([0.0, 0.0], Duration::ZERO);
    moves(
        &mut tracker,
        [0.0, 0.0],
        v,
        Duration::ZERO,
        Duration::from_millis(20),
        3,
    );
    tracker.up(Duration::from_millis(20));

    // Then
    assert_eq!(speed(&tracker, 100.0), [-100.0, 100.0]);
    assert_eq!(speed(&tracker, 1000.0), [-1000.0, 1000.0]);
}

#[test]
fn a_slower_stretch_lowers_the_velocity_and_a_faster_one_raises_it() {
    // Given Chromium's VaryingVelocity: 12 samples per 100 ms stretch
    let mut tracker = VelocityTracker::default();
    let (fast, slow, span) = ([0.0, 500.0], [0.0, 250.0], Duration::from_millis(100));
    tracker.down([0.0, 0.0], Duration::ZERO);
    moves(&mut tracker, [0.0, 0.0], fast, Duration::ZERO, span, 12);
    let after_fast = speed(&tracker, NO_LIMIT)[1];

    // When: a slow stretch, then a fast one
    moves(&mut tracker, [0.0, 50.0], slow, span, span, 12);
    let after_slow = speed(&tracker, NO_LIMIT)[1];
    moves(&mut tracker, [0.0, 75.0], fast, span * 2, span, 12);
    let after_fast_again = speed(&tracker, NO_LIMIT)[1];

    // Then
    assert!(
        after_slow < fast[1] && after_slow < after_fast,
        "{after_fast} then {after_slow}"
    );
    assert!(
        after_fast_again > slow[1] && after_fast_again > after_slow,
        "{after_slow} then {after_fast_again}"
    );
}

#[test]
fn an_up_80_ms_after_the_last_move_has_no_velocity() {
    // Given Chromium's DelayedActionUp: 10 samples over 20 ms
    let mut tracker = VelocityTracker::default();
    tracker.down([0.0, 0.0], Duration::ZERO);
    moves(
        &mut tracker,
        [0.0, 0.0],
        [-50_000.0, 50_000.0],
        Duration::ZERO,
        Duration::from_millis(20),
        10,
    );
    assert_eq!(speed(&tracker, 1000.0), [-1000.0, 1000.0]);

    // When: the up 100 ms after the movement's span
    tracker.up(Duration::from_millis(120));

    // Then
    assert_eq!(tracker.velocity(1000.0), Velocity::Stopped);
}

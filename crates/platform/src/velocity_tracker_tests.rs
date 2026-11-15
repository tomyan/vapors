//! The first four are Chromium's own tests of LSQ2
//! (`ui/events/velocity_tracker/velocity_tracker_unittest.cc`: `Basic`,
//! `MaxVelocity`, `VaryingVelocity`, `DelayedActionUp`) on the same synthetic
//! samples; the rest pin the history's rules.

use std::time::Duration;

use super::VelocityTracker;

const NO_LIMIT: f32 = 20_000.0;

fn at(v: [f32; 2], t: Duration) -> [f32; 2] {
    let s = t.as_secs_f32();
    [v[0] * s, v[1] * s]
}

/// `ApplyMovement`: `samples` moves at velocity `v` over `span` from
/// `start`, from the position at `start` (`from`).
fn moves(
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

fn near(got: f32, expected: f32, within: f32) {
    assert!(
        (got - expected).abs() <= within,
        "{got}, not {expected} (within {within})"
    );
}

#[test]
fn a_constant_velocity_is_measured_as_itself() {
    // Given Chromium's Basic: 60 samples over a second at (0, 500)
    let mut tracker = VelocityTracker::default();
    assert_eq!(tracker.velocity(NO_LIMIT), [0.0, 0.0]);
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
    let [x, y] = tracker.velocity(NO_LIMIT);
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
    assert_eq!(tracker.velocity(100.0), [-100.0, 100.0]);
    assert_eq!(tracker.velocity(1000.0), [-1000.0, 1000.0]);
}

#[test]
fn a_slower_stretch_lowers_the_velocity_and_a_faster_one_raises_it() {
    // Given Chromium's VaryingVelocity: 12 samples per 100 ms stretch
    let mut tracker = VelocityTracker::default();
    let (fast, slow, span) = ([0.0, 500.0], [0.0, 250.0], Duration::from_millis(100));
    tracker.down([0.0, 0.0], Duration::ZERO);
    moves(&mut tracker, [0.0, 0.0], fast, Duration::ZERO, span, 12);
    let after_fast = tracker.velocity(NO_LIMIT)[1];

    // When: a slow stretch, then a fast one
    moves(&mut tracker, [0.0, 50.0], slow, span, span, 12);
    let after_slow = tracker.velocity(NO_LIMIT)[1];
    moves(&mut tracker, [0.0, 75.0], fast, span * 2, span, 12);
    let after_fast_again = tracker.velocity(NO_LIMIT)[1];

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
    assert_eq!(tracker.velocity(1000.0), [-1000.0, 1000.0]);

    // When: the up 100 ms after the movement's span
    tracker.up(Duration::from_millis(120));

    // Then
    assert_eq!(tracker.velocity(1000.0), [0.0, 0.0]);
}

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
    near(tracker.velocity(NO_LIMIT)[1], 1000.0, 1.0);
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
    near(tracker.velocity(NO_LIMIT)[1], 200.0, 0.5);
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
    near(tracker.velocity(NO_LIMIT)[1], 500.0, 0.5);
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
    near(tracker.velocity(NO_LIMIT)[1], 1000.0, 0.5);
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
    near(tracker.velocity(NO_LIMIT)[1], 3280.0, 2.0);
}

#[test]
fn a_single_sample_has_no_velocity() {
    let mut tracker = VelocityTracker::default();
    tracker.down([3.0, 4.0], Duration::ZERO);
    assert_eq!(tracker.velocity(NO_LIMIT), [0.0, 0.0]);
}

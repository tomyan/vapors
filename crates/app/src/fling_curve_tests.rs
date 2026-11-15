use std::time::Duration;

use super::{FlingCurve, spline_position};

fn curve(velocity: [f32; 2]) -> FlingCurve {
    FlingCurve::new(velocity).expect("a moving fling")
}

#[test]
fn duration_and_distance_follow_chromiums_formulas() {
    // Given the worked values in docs/plans/touch-momentum.md: speed,
    // duration in ms, distance in CSS px
    let worked = [
        (1000.0, 555.18, 194.31),
        (3000.0, 1246.59, 1308.92),
        (8000.0, 2566.6, 7186.4),
    ];
    for (speed, millis, distance) in worked {
        // When
        let fling = curve([0.0, speed]);

        // Then
        let got = fling.duration().as_secs_f64() * 1000.0;
        assert!(
            (got - millis).abs() < 0.1,
            "{speed}: {got} ms, not {millis}"
        );
        assert!(
            (fling.distance() - distance).abs() < 0.1,
            "{speed}: {} px, not {distance}",
            fling.distance()
        );
    }
}

#[test]
fn spline_position_matches_chromiums_table() {
    // Given positions from the table as Chromium builds it (the same
    // bisection in scripts/fling-recorder/fit.py)
    let table = [
        (0.1, 0.274_002),
        (0.25, 0.583_811),
        (0.5, 0.858_411),
        (0.755, 0.972_295),
        (1.0, 1.0),
    ];
    for (u, position) in table {
        // When
        let got = spline_position(u);

        // Then
        assert!(
            (got - position).abs() < 1e-5,
            "at {u}: {got}, not {position}"
        );
    }
}

#[test]
fn offset_runs_along_the_velocity() {
    // Given a fling up and to the right, 3 across for 4 down
    let fling = curve([300.0, -400.0]);

    // When
    let [x, y] = fling
        .offset_at(fling.duration() / 4)
        .expect("still running");

    // Then
    assert!(x > 0.0 && y < 0.0, "{x}, {y}");
    assert!((x / y + 0.75).abs() < 1e-4, "{x}, {y}");
}

#[test]
fn offset_at_a_quarter_is_the_spline_there_times_the_distance() {
    // Given
    let fling = curve([0.0, 3000.0]);

    // When
    let [_, y] = fling
        .offset_at(fling.duration() / 4)
        .expect("still running");

    // Then
    let expected = spline_position(0.25) * fling.distance();
    assert!(
        (f64::from(y) - expected).abs() < 0.01,
        "{y}, not {expected}"
    );
}

#[test]
fn curve_ends_at_its_duration() {
    // Given
    let fling = curve([0.0, 1000.0]);

    // When
    let at_end = fling.offset_at(fling.duration());
    let just_before = fling.offset_at(fling.duration().mul_f64(0.9));

    // Then
    assert_eq!(at_end, None);
    assert!(just_before.is_some());
}

#[test]
fn curve_ends_within_a_tenth_of_a_pixel_of_its_end() {
    // Given a fling whose last 1 % of time covers under 0.1 px
    // (194.3 px × (1 − 0.99996))
    let fling = curve([0.0, 1000.0]);

    // When
    let late = fling.offset_at(fling.duration().mul_f64(0.99));

    // Then
    assert_eq!(late, None);
}

#[test]
fn still_or_unreadable_velocity_has_no_curve() {
    for velocity in [[0.0, 0.0], [f32::NAN, 10.0], [f32::INFINITY, 0.0]] {
        assert_eq!(FlingCurve::new(velocity), None, "{velocity:?}");
    }
}

#[test]
fn offset_at_the_start_is_zero() {
    // Given
    let fling = curve([0.0, 2000.0]);

    // When
    let [x, y] = fling.offset_at(Duration::ZERO).expect("running");

    // Then: the table's first sample is 2.3e-5, Chromium's bisection's
    assert_eq!(x, 0.0);
    assert!(y.abs() < 0.05, "{y}");
}

#[test]
fn velocity_starts_near_the_flings_and_falls_to_zero_at_its_end() {
    // Given a fling at 3000 px/s up and to the right, 3 across for 4 up
    let fling = curve([1800.0, -2400.0]);

    // When
    let [x, y] = fling.velocity_at(Duration::ZERO);
    let at_end = fling.velocity_at(fling.duration() + Duration::from_millis(1));

    // Then: the first segment's slope × distance / duration is within
    // 2% of the starting speed, along the fling's direction
    assert!((x.hypot(y) - 3000.0).abs() < 60.0, "{x}, {y}");
    assert!((x / y + 0.75).abs() < 1e-4, "{x}, {y}");
    assert_eq!(at_end, [0.0, 0.0]);
}

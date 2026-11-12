//! The circuit's centre line by distance from the start line.

use super::{LAP, distance_of, point_at};
use vapors_racer::model::{RADIUS, STRAIGHT};

fn close(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).hypot(a[1] - b[1]) < 0.01
}

#[test]
fn a_lap_is_four_half_straights_and_two_half_circles() {
    assert!((LAP - (4.0 * STRAIGHT + std::f32::consts::TAU * RADIUS)).abs() < 0.01);
}

#[test]
fn the_line_starts_at_the_car_and_turns_clockwise_at_each_end() {
    let half_bend = std::f32::consts::PI * RADIUS / 2.0;
    for (distance, point) in [
        (0.0, [0.0, -RADIUS]),
        (STRAIGHT, [STRAIGHT, -RADIUS]),
        (STRAIGHT + half_bend, [STRAIGHT + RADIUS, 0.0]),
        (STRAIGHT + 2.0 * half_bend, [STRAIGHT, RADIUS]),
        (LAP / 2.0, [0.0, RADIUS]),
        (LAP - STRAIGHT - half_bend, [-STRAIGHT - RADIUS, 0.0]),
        (LAP - 1.0, [-1.0, -RADIUS]),
        (LAP, [0.0, -RADIUS]),
    ] {
        let at = point_at(distance);
        assert!(
            close(at, point),
            "at {distance}: {at:?}, expected {point:?}"
        );
    }
}

#[test]
fn a_point_on_the_line_is_at_the_distance_it_was_taken_from() {
    for tenth in 0..10 {
        #[allow(clippy::cast_precision_loss)] // 0 to 9
        let distance = LAP * tenth as f32 / 10.0;
        let found = distance_of(point_at(distance));
        assert!((found - distance).abs() < 0.05, "{distance}: {found}");
    }
}

#[test]
fn a_point_off_the_line_is_at_the_distance_of_the_nearest_point_on_it() {
    // Given: inside the right bend and outside the bottom straight
    let inside = [STRAIGHT + 100.0, 0.0];
    let outside = [STRAIGHT / 2.0, RADIUS + 20.0];

    // Then
    let half_bend = std::f32::consts::PI * RADIUS / 2.0;
    assert!((distance_of(inside) - (STRAIGHT + half_bend)).abs() < 0.05);
    let bottom = STRAIGHT + 2.0 * half_bend + STRAIGHT / 2.0;
    assert!((distance_of(outside) - bottom).abs() < 0.05);
}

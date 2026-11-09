//! Driving and camera acceptance tests.
use vapors_racer::model::{Camera, Controls, Race, Settings};

#[test]
fn throttle_accelerates_and_brake_reduces_speed() {
    // Given
    let mut race = Race::default();
    // When
    for _ in 0..120 {
        race.step(
            1.0 / 120.0,
            Controls {
                throttle: 1.0,
                ..Controls::default()
            },
            Settings::default(),
        )
        .unwrap();
    }
    let fast = race.speed;
    for _ in 0..60 {
        race.step(
            1.0 / 120.0,
            Controls {
                brake: 1.0,
                ..Controls::default()
            },
            Settings::default(),
        )
        .unwrap();
    }
    // Then
    assert!(fast > 40.0);
    assert!(race.speed < fast);
}

#[test]
fn camera_widens_smoothly_as_speed_increases() {
    // Given
    let mut camera = Camera::default();
    let close = camera.zoom;
    // When
    camera.step(0.016, 220.0).unwrap();
    let first = camera.zoom;
    for _ in 0..180 {
        camera.step(0.016, 220.0).unwrap();
    }
    // Then
    assert!(first < close && first > 0.69);
    assert!(camera.zoom < 0.72);
    camera.step(0.016, 0.0).unwrap();
    assert!(camera.zoom > 0.68);
}

#[test]
fn invalid_simulation_input_is_refused_without_mutation() {
    // Given
    let mut race = Race::default();
    let initial = race.position;
    // When / Then
    for dt in [f32::NAN, -0.1, 1.0] {
        assert!(
            race.step(dt, Controls::default(), Settings::default())
                .is_err()
        );
        assert_eq!(race.position, initial);
    }
    assert!(
        race.step(
            0.01,
            Controls {
                throttle: 2.0,
                ..Controls::default()
            },
            Settings::default()
        )
        .is_err()
    );
    assert!(
        race.step(
            0.01,
            Controls::default(),
            Settings {
                grip: f32::NAN,
                ..Settings::default()
            }
        )
        .is_err()
    );
}

#[test]
fn grass_slows_the_car_more_than_tarmac() {
    // Given
    let mut road = Race::default();
    road.speed = 100.0;
    let mut grass = Race::default();
    grass.speed = 100.0;
    grass.position = [0.0, 0.0];
    // When
    for _ in 0..60 {
        road.step(1.0 / 120.0, Controls::default(), Settings::default())
            .unwrap();
        grass
            .step(1.0 / 120.0, Controls::default(), Settings::default())
            .unwrap();
    }
    // Then
    assert!(grass.speed < road.speed * 0.6);
}

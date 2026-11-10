//! Camera-mode transitions use world time and the shortest angular path.
use vapors_racer::model::Camera;
fn settle(camera: &mut Camera, heading: f32) {
    for _ in 0..240 {
        camera.step(1.0 / 60.0, 0.0, heading).unwrap();
    }
}
#[test]
fn follow_blends_to_a_closer_lower_view_ahead_of_the_car() {
    // Given
    let mut camera = Camera::default();
    // When
    camera.toggle();
    assert_eq!(camera.rotation, 0.0);
    camera.step(0.016, 0.0, 0.0).unwrap();
    // Then
    assert!(camera.rotation > 0.0 && camera.rotation < std::f32::consts::FRAC_PI_2);
    settle(&mut camera, 0.0);
    assert!((camera.rotation - std::f32::consts::FRAC_PI_2).abs() < 0.001);
    assert!(camera.zoom > 1.2 && camera.tilt < 0.7 && camera.offset[0] > 30.0);
    assert!(!camera.moving);
}
#[test]
fn following_across_heading_wrap_does_not_spin_the_view() {
    // Given
    let mut camera = Camera::default();
    camera.toggle();
    settle(&mut camera, std::f32::consts::TAU - 0.01);
    let before = camera.rotation;
    // When
    camera.step(0.016, 0.0, 0.01).unwrap();
    // Then
    assert!((camera.rotation - before).abs() < 0.01);
}
#[test]
fn switching_back_restores_fixed_orientation_and_scale() {
    // Given
    let mut camera = Camera::default();
    camera.toggle();
    settle(&mut camera, 1.0);
    // When
    camera.toggle();
    settle(&mut camera, 1.0);
    // Then
    assert!(camera.rotation.abs() < 0.001);
    assert!((camera.zoom - 1.0).abs() < 0.001);
    assert!((camera.tilt - 0.82).abs() < 0.001);
    assert!(camera.offset[0].abs() < 0.001 && camera.offset[1].abs() < 0.001);
    assert!(!camera.moving);
}
#[test]
fn invalid_heading_is_refused_before_changing_the_view() {
    // Given
    let mut camera = Camera::default();
    camera.toggle();
    // When / Then
    assert!(camera.step(0.016, 0.0, f32::NAN).is_err());
    assert_eq!(camera.rotation, 0.0);
    assert_eq!(camera.zoom, 1.0);
}

use super::*;

#[test]
fn a_wheel_position_is_the_client_position_from_the_canvas_corner_in_physical_pixels() {
    // Given: a canvas whose corner is at (10, 40) CSS px, at 2x
    // When
    let at = canvas_position([110.0, 90.0], [10.0, 40.0], 2.0);

    // Then
    assert_eq!(at, [200.0, 100.0]);
}

#[test]
fn a_recorded_position_is_taken_once() {
    // Given
    record([3.0, 4.0]);

    // When / Then
    assert_eq!(take(), Some([3.0, 4.0]));
    assert_eq!(take(), None);
}

use super::*;

#[test]
fn a_wheel_position_is_the_canvas_offset_in_physical_pixels() {
    // Given: an offset of (55.5, 25.25) CSS px from the canvas edge, at 2x
    // When
    let at = canvas_position([55.5, 25.25], 2.0);

    // Then: fractions survive, as in winit's pointer positions
    assert_eq!(at, [111.0, 50.5]);
}

#[test]
fn a_recorded_position_is_taken_once() {
    // Given
    record([3.0, 4.0]);

    // When / Then
    assert_eq!(take(), Some([3.0, 4.0]));
    assert_eq!(take(), None);
}

#[test]
fn clearing_drops_a_stale_position() {
    // Given
    record([3.0, 4.0]);

    // When
    clear();

    // Then
    assert_eq!(take(), None);
}

#[test]
fn the_listener_is_installed_once() {
    // Given / When
    let first = first_install();
    let second = first_install();

    // Then
    assert!(first);
    assert!(!second);
}

#[test]
fn a_missing_canvas_is_reported_once() {
    // Given / When
    let first = first_canvas_report();
    let second = first_canvas_report();

    // Then
    assert!(first);
    assert!(!second);
}

//! The status refuses a non-finite target rectangle, naming its field.
use super::inspection::rect_json;

#[test]
fn inspection_refuses_a_non_finite_target_rect_naming_the_field() {
    // Given
    let numbers = [0.0, f32::INFINITY, 10.0, 10.0];

    // When
    let rect = rect_json("gas", numbers);

    // Then
    assert!(rect.unwrap_err().contains("`targets.gas.y` is inf"));
}

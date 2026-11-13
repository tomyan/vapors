use super::half_up;

#[test]
fn an_edge_rounds_to_the_nearest_whole_pixel_a_half_up() {
    // Given, When, Then: each value and its snapped edge
    let cases = [
        (1.5, 2.0),
        (-1.5, -1.0),
        (-2.5, -2.0),
        (-0.5, 0.0),
        (2.4, 2.0),
        (-2.6, -3.0),
        (0.499_999_97, 0.0),
    ];
    for (value, edge) in cases {
        assert_eq!(half_up(value), edge, "{value}");
    }
}

#[test]
fn a_half_snapped_then_moved_by_a_whole_number_is_the_moved_half_snapped() {
    // Given: a tie below zero moved past it, as a scroll frame's shift
    // moves an edge snapped in the frame (iu4.md section 3)
    for (value, shift) in [(37.5_f32, -39.0_f32), (-1.5, 3.0), (2.5, -5.0)] {
        // When
        let shifted_after = half_up(value) + shift;

        // Then
        assert_eq!(shifted_after, half_up(value + shift), "{value} by {shift}");
    }
}

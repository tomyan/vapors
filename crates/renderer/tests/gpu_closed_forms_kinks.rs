//! GPU tier: coverage where the distance has a kink, its nearest edge
//! changing (a rect's diagonals and centre line, a circle's centre), is
//! still the distance in device pixels (#345 review, H1): a 1 px rule
//! covers two rows by their share, a fractional square corner is not
//! fully covered, a small circle's centre is.

#[path = "support/closed_forms.rs"]
mod support;

use support::{assert_at, circle, coverage, rect, scaled, white};

#[test]
fn gpu_a_one_pixel_rule_at_a_quarter_pixel_covers_two_rows_by_their_share() {
    // Given: a 1 px tall rect from y = 0.25 to 1.25
    let rule = rect([2.25, 0.25], [4.0, 1.0], 0.0);

    // When
    let coverage = coverage(&[white(&rule, None, scaled(1.0))]);

    // Then
    assert_at(&coverage, (4, 0), 0.75, "row 0, its centre 0.25 inside");
    assert_at(&coverage, (4, 1), 0.25, "row 1, its centre 0.25 outside");
    assert_at(
        &coverage,
        (2, 0),
        0.75,
        "the left end, 0.25 inside both edges",
    );
}

#[test]
fn gpu_a_fractional_square_corner_is_covered_by_its_distance() {
    // Given: a 6 x 6 square cornered rect from (2.25, 2.25)
    let square = rect([2.25, 2.25], [6.0, 6.0], 0.0);

    // When
    let coverage = coverage(&[white(&square, None, scaled(1.0))]);

    // Then
    assert_at(
        &coverage,
        (2, 2),
        0.75,
        "the corner pixel, 0.25 inside both edges",
    );
    assert_at(&coverage, (7, 2), 0.75, "the other top corner");
    assert_at(&coverage, (8, 2), 0.25, "0.25 past the right edge");
    assert_at(&coverage, (1, 1), 0.0, "outside the corner");
}

#[test]
fn gpu_a_small_circles_centre_is_covered() {
    // Given: a circle of radius 1 about a pixel centre
    let small = circle([4.5, 4.5], 1.0);

    // When
    let coverage = coverage(&[white(&small, None, scaled(1.0))]);

    // Then
    assert_at(&coverage, (4, 4), 1.0, "the centre");
    assert_at(&coverage, (5, 4), 0.5, "a pixel centre on the outline");
}

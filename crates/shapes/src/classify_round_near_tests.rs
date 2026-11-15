//! Near misses of a rounded rect: each is a path, and each is caught by
//! one guard of `classify_round` alone, so breaking that guard is the
//! only change that makes its test fail (#387 review, M2). The base is
//! SVG's outline of x=1 y=1 width=10 height=6, with rx=ry=2 or with
//! rx=3 ry=1.5.

use crate::canonical::{FillRule, Paint};
use crate::classify::{Classified, classify};
use crate::path::IconPath;

const FILL: Paint = Paint::Fill(FillRule::NonZero);
const STROKE: Paint = Paint::Stroke { width: 1.5 };

/// Asserts `data` under `paint` stays a path.
fn path(data: &str, paint: Paint) {
    // When
    let parsed = IconPath::parse(data).unwrap_or_else(|e| panic!("{data:?}: {e}"));
    let found = classify(&parsed, paint);

    // Then
    assert_eq!(found, Classified::Path, "{data}");
}

/// The elliptical base with its third arc written as `arc`.
fn elliptical_with(arc: &str) -> String {
    format!("M4 1H8A3 1.5 0 0 1 11 2.5V5.5A3 1.5 0 0 1 8 7H4{arc}V2.5A3 1.5 0 0 1 4 1Z")
}

#[test]
fn an_open_stroke_is_a_path() {
    // Given, Then: the outline's close
    path(
        "M3 1H9A2 2 0 0 1 11 3V5A2 2 0 0 1 9 7H3A2 2 0 0 1 1 5V3A2 2 0 0 1 3 1",
        STROKE,
    );
}

#[test]
fn a_second_subpath_is_a_path() {
    // Given, Then: the outline's count
    path(
        "M3 1H9A2 2 0 0 1 11 3V5A2 2 0 0 1 9 7H3A2 2 0 0 1 1 5V3A2 2 0 0 1 3 1ZM1 1h1v1z",
        FILL,
    );
}

#[test]
fn a_full_ellipse_with_its_sides_bunched_at_the_end_is_a_path() {
    // Given, Then: the alternation. Conservative: the four zero-length
    // sides after the arcs draw the same ellipse, but not as SVG writes it
    path(
        "M8 1A6 4 0 0 1 14 5A6 4 0 0 1 8 9A6 4 0 0 1 2 5A6 4 0 0 1 8 1H8V1H8V1Z",
        FILL,
    );
}

#[test]
fn a_curved_side_is_a_path() {
    // Given, Then: a cubic whose ends are the top side's
    path(
        "M3 1C5 0 7 0 9 1A2 2 0 0 1 11 3V5A2 2 0 0 1 9 7H3A2 2 0 0 1 1 5V3A2 2 0 0 1 3 1Z",
        FILL,
    );
}

#[test]
fn a_fill_whose_sides_drift_off_the_start_is_a_path() {
    // Given, Then: each side slants 0.00009 in x, within the tolerance
    // (1.1e-4 here), but the end lands 0.00018 off the start
    path(
        "M3 1H9A2 2 0 0 1 11 3L11.00009 5A2 2 0 0 1 9.00009 7H3.00009A2 2 0 0 1 1.00009 5L1.00018 3A2 2 0 0 1 3.00018 1Z",
        FILL,
    );
}

#[test]
fn a_stroke_that_ends_only_near_the_start_is_a_path() {
    // Given, Then: 0.00005 off, within the tolerance; its close would be
    // a segment with joins (#387 review, L2)
    path(
        "M3 1H9A2 2 0 0 1 11 3V5A2 2 0 0 1 9 7H3A2 2 0 0 1 1 5V3A2 2 0 0 1 3.00005 1Z",
        STROKE,
    );
}

#[test]
fn a_rotated_elliptical_arc_is_a_path() {
    // Given, Then: the third corner rotated 30 degrees; elliptical, so not
    // the same arc (#387 review, M2)
    path(&elliptical_with("A3 1.5 30 0 1 1 5.5"), FILL);
}

#[test]
fn a_large_arc_is_a_path() {
    // Given, Then
    path(&elliptical_with("A3 1.5 0 1 1 1 5.5"), FILL);
}

#[test]
fn one_corner_after_the_first_turned_in_is_a_path() {
    // Given, Then: the sweep the first arc sets
    path(&elliptical_with("A3 1.5 0 0 0 1 5.5"), FILL);
}

#[test]
fn every_corner_turned_in_is_a_path() {
    // Given, Then: the turn check
    path(
        "M3 1H9A2 2 0 0 0 11 3V5A2 2 0 0 0 9 7H3A2 2 0 0 0 1 5V3A2 2 0 0 0 3 1Z",
        FILL,
    );
}

#[test]
fn arcs_spanning_other_than_their_radii_are_a_path() {
    // Given, Then: the top side shifted 0.5 right
    path(
        "M2.5 1H8.5A2 2 0 0 1 11 3V5A2 2 0 0 1 9 7H3A2 2 0 0 1 1 5V3A2 2 0 0 1 2.5 1Z",
        FILL,
    );
}

#[test]
fn an_arc_of_another_radius_is_a_path() {
    // Given, Then: the third arc of radius 3 spanning 2 x 2
    path(
        "M3 1H9A2 2 0 0 1 11 3V5A2 2 0 0 1 9 7H3A3 3 0 0 1 1 5V3A2 2 0 0 1 3 1Z",
        FILL,
    );
}

#[test]
fn a_sloping_side_is_a_path() {
    // Given, Then: the top side sloping 0.2, its right side short to match
    path(
        "M3 1L9 1.2A2 2 0 0 1 11 3.2V5A2 2 0 0 1 9 7H3A2 2 0 0 1 1 5V3A2 2 0 0 1 3 1Z",
        FILL,
    );
}

#[test]
fn corners_below_the_radius_floor_are_a_path() {
    // Given, Then: geometry::MIN_RADIUS
    path(
        "M1e-10 0H3e-10A1e-10 1e-10 0 0 1 4e-10 1e-10V3e-10A1e-10 1e-10 0 0 1 3e-10 4e-10H1e-10A1e-10 1e-10 0 0 1 0 3e-10V1e-10A1e-10 1e-10 0 0 1 1e-10 0Z",
        FILL,
    );
}

#[test]
fn a_sloping_side_a_millionth_the_size_is_a_path() {
    // Given, Then: the side check, its tolerance relative to the shape,
    // not 1e-5 absolute (#387 review, L1)
    path(
        "M3e-6 1e-6L9e-6 1.2e-6A2e-6 2e-6 0 0 1 11e-6 3.2e-6V5e-6A2e-6 2e-6 0 0 1 9e-6 7e-6H3e-6A2e-6 2e-6 0 0 1 1e-6 5e-6V3e-6A2e-6 2e-6 0 0 1 3e-6 1e-6Z",
        FILL,
    );
}

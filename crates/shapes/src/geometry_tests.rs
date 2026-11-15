//! Each closed form the check refuses, and the ones it passes; meshes
//! are checked in `mesh_tests.rs`.

use super::*;

fn rect(size: [f32; 2], radius: [f32; 2]) -> Geometry<'static> {
    Geometry::Rect {
        origin: [1.0, 1.0],
        size,
        radius,
    }
}

fn ellipse(centre: [f32; 2], radii: [f32; 2]) -> Geometry<'static> {
    Geometry::Ellipse { centre, radii }
}

fn refused(geometry: &Geometry, reason: &str) {
    assert_eq!(
        check_geometry(geometry).as_ref().map_err(String::as_str),
        Err(reason),
        "{geometry:?}"
    );
}

#[test]
fn drawable_geometries_pass() {
    // Given
    let drawable = [
        rect([2.0, 3.0], [0.0, 0.0]),
        rect([2.0, 3.0], [1.0, 1.0]),
        rect([2.0, 3.0], [0.5, 1.0]),
        ellipse([8.0, 8.0], [3.0, 3.0]),
    ];

    // When, Then
    for geometry in &drawable {
        assert_eq!(check_geometry(geometry), Ok(()), "{geometry:?}");
    }
}

#[test]
fn a_closed_form_with_a_non_finite_or_non_positive_value_is_refused() {
    // Given
    let cases = [
        (
            rect([f32::NAN, 1.0], [0.0; 2]),
            "size [NaN, 1.0] is not a finite number",
        ),
        (
            rect([2.0, 0.0], [0.0; 2]),
            "size [2.0, 0.0] is not above zero",
        ),
        (
            Geometry::Rect {
                origin: [f32::INFINITY, 0.0],
                size: [1.0; 2],
                radius: [0.0; 2],
            },
            "origin [inf, 0.0] is not a finite number",
        ),
        (
            rect([2.0, 2.0], [f32::NAN, f32::NAN]),
            "radius [NaN, NaN] is not a finite number",
        ),
        (
            ellipse([f32::NEG_INFINITY, 0.0], [1.0; 2]),
            "centre [-inf, 0.0] is not a finite number",
        ),
        (
            ellipse([0.0; 2], [1.0, -1.0]),
            "radii [1.0, -1.0] is not above zero",
        ),
    ];

    // When, Then
    for (geometry, reason) in &cases {
        refused(geometry, reason);
    }
}

#[test]
fn a_corner_radius_the_box_shader_cannot_draw_is_refused() {
    // Given
    let cases = [
        (
            rect([2.0, 2.0], [-1.0, -1.0]),
            "a corner radius of [-1.0, -1.0] is negative",
        ),
        (
            rect([10.0, 4.0], [6.0, 6.0]),
            "a corner radius of [6.0, 6.0] is over half the size [10.0, 4.0]; the emitter clamps rx to width / 2 and ry to height / 2, as SVG does",
        ),
        (
            rect([10.0, 4.0], [1.0, 2.5]),
            "a corner radius of [1.0, 2.5] is over half the size [10.0, 4.0]; the emitter clamps rx to width / 2 and ry to height / 2, as SVG does",
        ),
        (
            rect([4.0, 10.0], [3.0, 3.0]),
            "a corner radius of [3.0, 3.0] is over half the size [4.0, 10.0]; the emitter clamps rx to width / 2 and ry to height / 2, as SVG does",
        ),
        (
            rect([10.0, 4.0], [3.0, 3.0]),
            "a corner radius of [3.0, 3.0] is over half the size [10.0, 4.0]; the emitter clamps rx to width / 2 and ry to height / 2, as SVG does",
        ),
        (
            rect([2.0, 3.0], [0.0, 1.0]),
            "corner radii [0.0, 1.0] have one zero; SVG draws square corners when either is zero, so the emitter writes both zero",
        ),
    ];

    // When, Then
    for (geometry, reason) in &cases {
        refused(geometry, reason);
    }
}

#[test]
fn a_radius_above_zero_too_small_for_the_shader_is_refused() {
    // Given: a corner radius and an ellipse radius just under the floor,
    // and both at it, which pass
    let tiny = MIN_RADIUS / 2.0;
    let reason = |name: &str, values: [f32; 2]| {
        format!(
            "{name} {values:?} include one below {MIN_RADIUS}, which the shader's arithmetic underflows; draw it as a square corner or a point, or scale the viewBox up"
        )
    };

    // When, Then
    refused(
        &rect([2.0, 3.0], [tiny, 1.0]),
        &reason("corner radii", [tiny, 1.0]),
    );
    refused(
        &ellipse([8.0, 8.0], [3.0, tiny]),
        &reason("radii", [3.0, tiny]),
    );
    assert_eq!(check_geometry(&rect([2.0, 3.0], [MIN_RADIUS; 2])), Ok(()));
    assert_eq!(
        check_geometry(&ellipse([8.0, 8.0], [MIN_RADIUS; 2])),
        Ok(())
    );
}

#[test]
fn a_radius_over_half_its_side_by_less_than_the_tolerance_is_clamped() {
    // Given: the pill <rect x=1 y=0.05 width=10 height=1 rx=0.5> as f32
    // gives it (#387 review, H1), and one over by 0.01
    let height = 1.05f32 - 0.05;

    // When
    let fitted = fitted_radii([1.0, 0.05], [10.0, height], [0.5, 0.5]);
    let over = fitted_radii([1.0, 0.05], [10.0, 1.0], [0.5, 0.51]);

    // Then
    assert_eq!(fitted, Ok([0.5, height / 2.0]));
    assert!(over.is_err(), "{over:?}");
    let pill = Geometry::Rect {
        origin: [1.0, 0.05],
        size: [10.0, height],
        radius: [0.5, 0.5],
    };
    assert_eq!(check_geometry(&pill), Ok(()));
}

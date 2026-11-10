use super::*;
use std::f32::consts::FRAC_PI_2;

/// A camera 10 units up +Z, looking at the origin with +Y up.
fn front() -> View {
    View::look_at(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::Y).unwrap()
}

fn ndc(projection: Projection, aspect: f32, point: Vec3) -> Vec3 {
    let matrix = view_projection(&projection, &front(), aspect).unwrap();
    project(matrix, point).expect("in front of the eye")
}

#[test]
fn the_target_projects_to_the_centre_and_x_right_y_up_on_screen() {
    // Given
    for projection in [
        Projection::perspective(FRAC_PI_2, 1.0, 100.0),
        Projection::orthographic(4.0, 1.0, 100.0),
    ] {
        let projection = projection.unwrap();
        assert!(
            ndc(projection, 1.0, Vec3::ZERO)
                .truncate()
                .abs_diff_eq(glam::Vec2::ZERO, 1e-6)
        );
        assert!(ndc(projection, 1.0, Vec3::X).x > 0.0);
        assert!(ndc(projection, 1.0, Vec3::Y).y > 0.0);
    }
}

#[test]
fn perspective_depth_runs_from_0_at_near_to_1_at_far() {
    // Given
    let projection = Projection::perspective(FRAC_PI_2, 1.0, 100.0).unwrap();

    // When
    let at_near = ndc(projection, 1.0, Vec3::new(0.0, 0.0, 9.0)).z;
    let at_far = ndc(projection, 1.0, Vec3::new(0.0, 0.0, -90.0)).z;

    // Then
    assert!(at_near.abs() < 1e-6, "{at_near}");
    assert!((at_far - 1.0).abs() < 1e-5, "{at_far}");
}

#[test]
fn a_90_degree_perspective_puts_the_top_edge_one_distance_up() {
    // Given: 10 units from the eye, a 90 degree field of view spans 10 up
    let projection = Projection::perspective(FRAC_PI_2, 1.0, 100.0).unwrap();

    // Then: y = 1 at the top edge; at aspect 2 the same x lands at 0.5
    assert!((ndc(projection, 1.0, Vec3::new(0.0, 10.0, 0.0)).y - 1.0).abs() < 1e-5);
    assert!((ndc(projection, 2.0, Vec3::new(10.0, 0.0, 0.0)).x - 0.5).abs() < 1e-5);
}

#[test]
fn an_orthographic_projection_spans_its_height_whatever_the_distance() {
    // Given
    let projection = Projection::orthographic(4.0, 0.0, 100.0).unwrap();

    // When / Then
    for z in [5.0, -50.0] {
        assert!((ndc(projection, 1.0, Vec3::new(0.0, 2.0, z)).y - 1.0).abs() < 1e-5);
        assert!((ndc(projection, 2.0, Vec3::new(4.0, 0.0, z)).x - 1.0).abs() < 1e-5);
    }
}

#[test]
fn a_point_at_or_behind_the_eye_has_no_position_under_perspective() {
    // Given: the eye at z = 10
    let projection = Projection::perspective(FRAC_PI_2, 1.0, 100.0).unwrap();
    let matrix = view_projection(&projection, &front(), 1.0).unwrap();

    // When
    let at_eye = project(matrix, Vec3::new(0.0, 0.0, 10.0));
    let behind = project(matrix, Vec3::new(0.0, 0.0, 20.0));

    // Then
    assert_eq!((at_eye, behind), (None, None));
}

#[test]
fn orthographic_depth_runs_from_0_at_near_to_1_at_far() {
    // Given: near behind the eye (negative), the eye 10 from the origin
    let projection = Projection::orthographic(4.0, -5.0, 95.0).unwrap();

    // When / Then: view distance -5 (z = 15) is 0, 5 (z = 5) is 0.1, 95 is 1
    for (z, depth) in [(15.0, 0.0), (5.0, 0.1), (-85.0, 1.0)] {
        let actual = ndc(projection, 1.0, Vec3::new(0.0, 0.0, z)).z;
        assert!((actual - depth).abs() < 1e-5, "z {z}: {actual}");
    }
}

#[test]
fn orthographic_depth_with_near_zero_starts_at_the_eye_plane() {
    // Given
    let projection = Projection::orthographic(4.0, 0.0, 100.0).unwrap();

    // When / Then: the eye plane (z = 10) is 0, the origin 0.1, z = -90 is 1
    for (z, depth) in [(10.0, 0.0), (0.0, 0.1), (-90.0, 1.0)] {
        let actual = ndc(projection, 1.0, Vec3::new(0.0, 0.0, z)).z;
        assert!((actual - depth).abs() < 1e-5, "z {z}: {actual}");
    }
}

#[test]
fn a_point_behind_the_eye_still_projects_under_orthographic_with_depth_outside_0_to_1() {
    // Given: the eye at z = 10, near -5
    let projection = Projection::orthographic(4.0, -5.0, 95.0).unwrap();
    let matrix = view_projection(&projection, &front(), 1.0).unwrap();

    // When: a point 10 behind the eye
    let point = project(matrix, Vec3::new(0.0, 0.0, 20.0));

    // Then: w is 1, so it projects; depth (-10 + 5) / 100 is below 0
    let depth = point.expect("orthographic projects every point").z;
    assert!((depth + 0.05).abs() < 1e-5, "{depth}");
}

#[test]
fn a_perspective_field_of_view_that_overflows_the_matrix_is_refused() {
    // Given / When: 1e-39 is finite and in (0, pi), and its cotangent is inf
    let error = Projection::perspective(1e-39, 1.0, 10.0).unwrap_err();

    // Then
    assert!(
        matches!(error, SceneError::MatrixNotFinite { .. }),
        "{error}"
    );
    assert!(error.to_string().contains("vertical_fov: 1e-39"), "{error}");
}

#[test]
fn an_orthographic_height_that_overflows_the_matrix_is_refused() {
    // Given / When: 1e-45 is finite and positive, and 2 / height is inf
    let error = Projection::orthographic(1e-45, 0.0, 10.0).unwrap_err();

    // Then
    assert!(
        matches!(error, SceneError::MatrixNotFinite { .. }),
        "{error}"
    );
    assert!(error.to_string().contains("height: 1e-45"), "{error}");
}

#[test]
fn an_aspect_that_overflows_the_matrix_is_refused() {
    // Given: a valid projection
    let projection = Projection::perspective(1.0, 1.0, 10.0).unwrap();

    // When: 1e-39 is positive and finite, and 1 / (tan * aspect) is inf
    let error = projection.matrix(1e-39).unwrap_err();

    // Then
    assert!(
        matches!(error, SceneError::MatrixNotFinite { .. }),
        "{error}"
    );
    assert!(error.to_string().contains("aspect 0.000"), "{error}");
}

#[test]
fn invalid_projections_are_refused_naming_the_fix() {
    // Given
    let perspective = Projection::perspective(1.0, 1.0, 10.0).unwrap();
    let cases: [(Result<(), SceneError>, &str); 8] = [
        (
            Projection::perspective(0.0, 1.0, 10.0).map(drop),
            "Projection::perspective vertical_fov is 0 radians",
        ),
        (
            Projection::perspective(PI, 1.0, 10.0).map(drop),
            "Projection::perspective vertical_fov is 3.1415",
        ),
        (
            Projection::perspective(f32::NAN, 1.0, 10.0).map(drop),
            "Projection::perspective vertical_fov is NaN",
        ),
        (
            Projection::perspective(1.0, 0.0, 10.0).map(drop),
            "Projection::perspective near is 0",
        ),
        (
            Projection::perspective(1.0, 1.0, 1.0).map(drop),
            "Projection far is 1, not beyond near 1",
        ),
        (
            Projection::orthographic(-1.0, 0.0, 10.0).map(drop),
            "Projection::orthographic height is -1",
        ),
        (
            Projection::orthographic(1.0, 0.0, f32::INFINITY).map(drop),
            "Projection far is NaN or infinite",
        ),
        (
            perspective.matrix(0.0).map(drop),
            "the viewport aspect ratio is 0",
        ),
    ];

    // When / Then
    for (result, message) in cases {
        let error = result.expect_err(message).to_string();
        assert!(error.starts_with(message), "{error}");
    }
}

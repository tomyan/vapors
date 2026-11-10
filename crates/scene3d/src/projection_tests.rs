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
    let projection = Projection::perspective(FRAC_PI_2, 1.0, 100.0).unwrap();
    assert!((ndc(projection, 1.0, Vec3::new(0.0, 0.0, 9.0)).z).abs() < 1e-6);
    assert!((ndc(projection, 1.0, Vec3::new(0.0, 0.0, -90.0)).z - 1.0).abs() < 1e-5);
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
    let projection = Projection::orthographic(4.0, 0.0, 100.0).unwrap();
    for z in [5.0, -50.0] {
        assert!((ndc(projection, 1.0, Vec3::new(0.0, 2.0, z)).y - 1.0).abs() < 1e-5);
        assert!((ndc(projection, 2.0, Vec3::new(4.0, 0.0, z)).x - 1.0).abs() < 1e-5);
    }
}

#[test]
fn a_point_at_or_behind_the_eye_has_no_position_under_perspective() {
    let projection = Projection::perspective(FRAC_PI_2, 1.0, 100.0).unwrap();
    let matrix = view_projection(&projection, &front(), 1.0).unwrap();
    assert_eq!(project(matrix, Vec3::new(0.0, 0.0, 10.0)), None);
    assert_eq!(project(matrix, Vec3::new(0.0, 0.0, 20.0)), None);
}

#[test]
fn invalid_projections_and_views_are_refused_naming_the_fix() {
    let perspective = Projection::perspective(1.0, 1.0, 10.0).unwrap();
    let cases: [(Result<(), SceneError>, &str); 11] = [
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
        (
            View::look_at(Vec3::ONE, Vec3::ONE, Vec3::Y).map(drop),
            "View::look_at eye equals target",
        ),
        (
            View::look_at(Vec3::Y, Vec3::ZERO, Vec3::Y).map(drop),
            "View::look_at up is zero or parallel",
        ),
        (
            View::look_at(Vec3::ZERO, Vec3::new(0.0, f32::NAN, 1.0), Vec3::Y).map(drop),
            "View::target is NaN",
        ),
    ];
    for (result, message) in cases {
        let error = result.expect_err(message).to_string();
        assert!(error.starts_with(message), "{error}");
    }
}

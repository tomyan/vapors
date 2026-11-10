use super::*;

fn refused(eye: Vec3, target: Vec3, up: Vec3) -> String {
    View::look_at(eye, target, up).unwrap_err().to_string()
}

#[test]
fn a_view_maps_its_eye_to_the_origin_and_its_target_down_minus_z() {
    // Given
    let view = View::look_at(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::Y).unwrap();

    // When
    let eye = view.matrix().transform_point3(Vec3::new(0.0, 0.0, 10.0));
    let target = view.matrix().transform_point3(Vec3::ZERO);

    // Then
    assert!(eye.abs_diff_eq(Vec3::ZERO, 1e-6), "{eye}");
    assert!(
        target.abs_diff_eq(Vec3::new(0.0, 0.0, -10.0), 1e-5),
        "{target}"
    );
}

#[test]
fn an_up_of_any_length_gives_the_same_view() {
    // Given
    let eye = Vec3::new(3.0, 4.0, 5.0);

    // When
    let unit = View::look_at(eye, Vec3::ZERO, Vec3::Y).unwrap();
    let long = View::look_at(eye, Vec3::ZERO, Vec3::Y * 1000.0).unwrap();

    // Then
    assert!(unit.matrix().abs_diff_eq(long.matrix(), 1e-5));
}

#[test]
fn an_eye_at_its_target_is_refused() {
    assert!(refused(Vec3::ONE, Vec3::ONE, Vec3::Y).starts_with("View::look_at eye equals target"));
}

#[test]
fn an_up_parallel_to_the_direction_is_refused() {
    assert!(
        refused(Vec3::Y, Vec3::ZERO, Vec3::Y).starts_with("View::look_at up is zero or parallel")
    );
}

#[test]
fn a_zero_up_is_refused() {
    assert!(
        refused(Vec3::Z, Vec3::ZERO, Vec3::ZERO)
            .starts_with("View::look_at up is zero or parallel")
    );
}

#[test]
fn an_up_nearly_parallel_to_the_direction_is_refused() {
    // Given: a direction -Y and an up leaning 1e-7 off it, under the 1e-6 sine
    let up = Vec3::new(1e-7, -1.0, 0.0);

    // When / Then
    assert!(refused(Vec3::Y, Vec3::ZERO, up).starts_with("View::look_at up is zero or parallel"));
}

#[test]
fn an_up_just_across_the_tolerance_is_accepted() {
    // Given: a sine of 1e-5, ten times the tolerance
    let up = Vec3::new(1e-5, -1.0, 0.0);

    // When / Then
    assert!(View::look_at(Vec3::Y, Vec3::ZERO, up).is_ok());
}

#[test]
fn a_non_finite_target_is_refused_naming_the_field() {
    let error = refused(Vec3::ZERO, Vec3::new(0.0, f32::NAN, 1.0), Vec3::Y);
    assert!(error.starts_with("View::target is NaN"), "{error}");
}

#[test]
fn finite_inputs_whose_matrix_overflows_are_refused_naming_them() {
    // Given: a direction of 2e19, whose squared length passes f32::MAX
    let target = Vec3::new(0.0, 0.0, -2e19);

    // When
    let error = View::look_at(Vec3::ZERO, target, Vec3::Y).unwrap_err();

    // Then
    assert!(
        matches!(error, SceneError::MatrixNotFinite { .. }),
        "{error}"
    );
    assert!(error.to_string().contains("target"), "{error}");
}

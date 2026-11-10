use super::*;
use std::f32::consts::FRAC_PI_2;

#[test]
fn a_transform_scales_then_rotates_then_translates() {
    // Given: scale 2, a quarter turn about +Y, then 5 along +Z
    let rotation = Quat::from_axis_angle(Vec3::Y, FRAC_PI_2);
    let transform = Transform::new(Vec3::new(0.0, 0.0, 5.0), rotation, Vec3::splat(2.0)).unwrap();

    // When
    let point = transform.matrix().transform_point3(Vec3::X);

    // Then: +X scaled to 2, turned to -Z (right-handed), moved to z = 3
    assert!(point.abs_diff_eq(Vec3::new(0.0, 0.0, 3.0), 1e-5), "{point}");
}

#[test]
fn the_identity_leaves_a_point_where_it_is() {
    let point = Vec3::new(1.0, 2.0, 3.0);
    assert_eq!(Transform::IDENTITY.matrix().transform_point3(point), point);
}

#[test]
fn invalid_transforms_are_refused_naming_the_field() {
    let unit = Quat::IDENTITY;
    let cases = [
        (
            Transform::new(Vec3::new(f32::NAN, 0.0, 0.0), unit, Vec3::ONE),
            "Transform::translation is NaN or infinite",
        ),
        (
            Transform::new(Vec3::ZERO, unit, Vec3::new(1.0, f32::INFINITY, 1.0)),
            "Transform::scale is NaN or infinite",
        ),
        (
            Transform::new(
                Vec3::ZERO,
                Quat::from_xyzw(f32::NAN, 0.0, 0.0, 1.0),
                Vec3::ONE,
            ),
            "Transform::rotation is NaN or infinite",
        ),
        (
            Transform::new(Vec3::ZERO, Quat::from_xyzw(0.0, 0.0, 0.0, 2.0), Vec3::ONE),
            "Transform::rotation has length 2, not 1",
        ),
        (
            Transform::new(Vec3::ZERO, unit, Vec3::new(1.0, 1.0, 0.0)),
            "Transform::scale.z is 0",
        ),
        (
            Transform::from_translation(Vec3::splat(f32::NEG_INFINITY)),
            "Transform::translation is NaN or infinite",
        ),
    ];
    for (result, message) in cases {
        let error = result.expect_err(message).to_string();
        assert!(error.starts_with(message), "{error}");
    }
}

#[test]
fn a_rotation_composed_in_f32_still_counts_as_unit() {
    // Given: a few hundred small turns, as an animation accumulates
    let step = Quat::from_axis_angle(Vec3::new(1.0, 2.0, 3.0).normalize(), 0.01);
    let rotation = (0..500).fold(Quat::IDENTITY, |acc, _| acc * step);

    // Then
    assert!(Transform::new(Vec3::ZERO, rotation, Vec3::ONE).is_ok());
}

#[test]
fn a_negative_scale_mirrors_and_is_allowed() {
    let mirror = Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::new(-1.0, 1.0, 1.0)).unwrap();
    assert_eq!(mirror.matrix().transform_point3(Vec3::X), -Vec3::X);
}

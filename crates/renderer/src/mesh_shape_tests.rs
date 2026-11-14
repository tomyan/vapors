//! A mesh item: the inverse of its determinant for the vertex shader;
//! a transform with no area draws nothing; one not finite, or whose
//! inverse is not, is refused.

use vapors_scene::Color;

use super::MeshShape;

const RED: Color = Color {
    r: 1.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

#[test]
fn an_instance_carries_the_transform_and_the_inverse_of_its_determinant() {
    // When
    let shape = MeshShape::new([2.0, 0.0, 1.0, 4.0, 5.0, 6.0], RED)
        .expect("drawable")
        .expect("has area");

    // Then
    let expected = [2.0, 0.0, 1.0, 4.0, 5.0, 6.0, 0.125, 0.0, 1.0, 0.0, 0.0, 1.0];
    assert_eq!(shape.instance(), expected);
}

#[test]
fn a_transform_with_no_area_draws_nothing_and_one_not_finite_is_refused() {
    // When, Then
    assert_eq!(MeshShape::new([0.0; 6], RED), Ok(None));
    let refused =
        MeshShape::new([f32::INFINITY, 0.0, 0.0, 1.0, 0.0, 0.0], RED).expect_err("refused");
    assert!(
        refused
            .0
            .starts_with("transform [inf, 0.0, 0.0, 1.0, 0.0, 0.0] is not finite"),
        "{refused}"
    );
}

#[test]
fn a_transform_whose_inverse_overflows_is_refused_naming_it() {
    // Given: finite values and a finite, subnormal determinant whose
    // inverse is past f32
    let tiny = [1.0, 0.0, 0.0, 1e-39, 0.0, 0.0];

    // When
    let refused = MeshShape::new(tiny, RED).expect_err("refused");

    // Then
    assert_eq!(
        refused.0,
        "transform [1.0, 0.0, 0.0, 1e-39, 0.0, 0.0] has no finite inverse: its scale is too small for f32; give the element a transform whose scale is within f32's range; reproduce with `cargo test -p vapors-renderer --lib mesh_shape`"
    );
}

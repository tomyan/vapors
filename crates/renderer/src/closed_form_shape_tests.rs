//! What the closed-form pipeline draws: geometry the shapes crate's
//! check passes, a stroke width above zero, a finite transform; a
//! transform with no area draws nothing.

use vapors_scene::Color;
use vapors_shapes::{FillRule, Geometry, IconPath, Paint, tessellate};

use super::{ClosedFormError, ClosedFormShape};

const CIRCLE: Geometry = Geometry::Ellipse {
    centre: [4.0, 4.0],
    radii: [2.0, 2.0],
};
const IDENTITY: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

fn shape(
    geometry: &Geometry,
    stroke: Option<f32>,
    transform: [f32; 6],
) -> Result<Option<ClosedFormShape>, ClosedFormError> {
    ClosedFormShape::new(geometry, stroke, transform, Color::from_rgba8(1, 2, 3, 255))
}

fn refused(result: Result<Option<ClosedFormShape>, ClosedFormError>) -> String {
    result.expect_err("refused").0
}

#[test]
fn a_stroke_width_that_is_not_above_zero_is_refused_naming_it() {
    // When, Then: none of them draws a fill instead
    for width in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert_eq!(
            refused(shape(&CIRCLE, Some(width), IDENTITY)),
            format!(
                "stroke width {width} is not a finite number above zero; give the shape a stroke-width above zero; reproduce with `cargo test -p vapors-renderer --lib closed_form_shape`"
            )
        );
    }
}

#[test]
fn geometry_the_shapes_check_refuses_is_refused_with_its_reason() {
    // Given
    let flat = Geometry::Ellipse {
        centre: [4.0, 4.0],
        radii: [2.0, 0.0],
    };

    // When, Then
    assert_eq!(
        refused(shape(&flat, None, IDENTITY)),
        "radii [2.0, 0.0] is not above zero; the shape table's check refuses this geometry, so fix the emitter that wrote it; reproduce with `cargo test -p vapors-renderer --lib closed_form_shape`"
    );
}

#[test]
fn a_mesh_is_refused_as_the_mesh_pipelines() {
    // Given: a triangle as the tessellator builds it
    let path = IconPath::parse("M0 0L4 0L0 4z").expect("parses");
    let mesh =
        tessellate("a.wisp:3", &path, Paint::Fill(FillRule::NonZero), 0.05).expect("tessellated");
    let mesh = Geometry::Mesh {
        triangles: Box::leak(mesh.triangles.into_boxed_slice()),
    };

    // When, Then
    assert_eq!(
        refused(shape(&mesh, None, IDENTITY)),
        "a mesh is not a closed form; draw a mesh entry with the mesh pipeline (one-scene S3a2); reproduce with `cargo test -p vapors-renderer --lib closed_form_shape`"
    );
}

#[test]
fn a_transform_with_a_value_that_is_not_finite_is_refused() {
    // When, Then
    assert_eq!(
        refused(shape(&CIRCLE, None, [1.0, 0.0, 0.0, f32::NAN, 0.0, 0.0])),
        "transform [1.0, 0.0, 0.0, NaN, 0.0, 0.0] is not finite; give the element a transform whose values and scale are finite; reproduce with `cargo test -p vapors-renderer --lib closed_form_shape`"
    );
}

#[test]
fn a_transform_whose_determinant_is_not_finite_is_refused() {
    // When: each value finite, the scale past f32
    let huge = [1e30, 0.0, 0.0, 1e30, 0.0, 0.0];

    // Then: not taken for a transform with area, nor for none
    assert_eq!(
        refused(shape(&CIRCLE, None, huge)),
        "transform [1e30, 0.0, 0.0, 1e30, 0.0, 0.0] has determinant inf, which is not finite; give the element a transform whose values and scale are finite; reproduce with `cargo test -p vapors-renderer --lib closed_form_shape`"
    );
}

#[test]
fn a_transform_with_no_area_draws_nothing() {
    // When, Then: CSS's scale(0) and a scale to a line
    assert_eq!(shape(&CIRCLE, None, [0.0; 6]), Ok(None));
    assert_eq!(
        shape(&CIRCLE, None, [1.0, 2.0, 2.0, 4.0, 0.0, 0.0]),
        Ok(None)
    );
}

#[test]
fn a_shape_is_five_vec4s_with_its_stroke_flagged() {
    // When
    let stroked = shape(&CIRCLE, Some(0.5), [2.0, 0.0, 0.0, 3.0, 5.0, 6.0])
        .expect("drawable")
        .expect("has area");
    let filled = shape(&CIRCLE, None, IDENTITY)
        .expect("drawable")
        .expect("has area");

    // Then
    let colour = Color::from_rgba8(1, 2, 3, 255);
    let expected = [
        2.0, 0.0, 0.0, 3.0, 5.0, 6.0, 1.0, 0.5, 4.0, 4.0, 2.0, 2.0, 0.0, 1.0, 0.0, 0.0, colour.r,
        colour.g, colour.b, colour.a,
    ];
    assert_eq!(stroked.instance(), expected);
    assert_eq!(filled.instance()[13], 0.0, "filled");
}

//! A mesh under a translated and rotated group lands where
//! `vapors_scene3d::project` puts it, under both projections.

mod support;

use std::f32::consts::FRAC_PI_2;
use support::{Fixture, RED, SIZE, centroid, render, to_pixel};
use vapors_scene3d::{Projection, Quat, Transform, Vec3, View, project, view_projection};

const EYE: Vec3 = Vec3::new(0.0, 0.0, 5.0);

/// Renders a red square at local (1, 0, 0) under a group at (0.5, 0, 0)
/// turned a quarter about Z, so its centre is at world (0.5, 1, 0); and
/// returns where it landed and where `project` puts that point.
fn placed(projection: Projection) -> ([f32; 2], [f32; 2]) {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let fixture = Fixture::new();
    fixture.camera(projection, EYE);
    let turn = Quat::from_rotation_z(FRAC_PI_2);
    let group_at = Transform::new(Vec3::new(0.5, 0.0, 0.0), turn, Vec3::ONE).unwrap();
    let group = fixture.group(group_at);
    let square = fixture.square(0.2);
    fixture.mesh(
        group,
        square,
        RED,
        Transform::from_translation(Vec3::X).unwrap(),
    );
    let mut renderer = fixture.renderer(&gpu);

    // When
    let pixels = render(&mut renderer, &gpu).expect("renders");

    // Then
    let (landed, count) = centroid(&pixels, RED);
    assert!(count > 20, "the square covers {count} pixels");
    let view = View::look_at(EYE, Vec3::ZERO, Vec3::Y).unwrap();
    #[allow(clippy::cast_precision_loss)]
    let aspect = SIZE.width as f32 / SIZE.height as f32;
    let expected = project(
        view_projection(&projection, &view, aspect).unwrap(),
        Vec3::new(0.5, 1.0, 0.0),
    );
    (landed, to_pixel(expected.expect("in front of the eye")))
}

fn assert_within_a_pixel(landed: [f32; 2], expected: [f32; 2]) {
    let distance = ((landed[0] - expected[0]).powi(2) + (landed[1] - expected[1]).powi(2)).sqrt();
    assert!(
        distance <= 1.0,
        "landed at {landed:?}, project gives {expected:?}"
    );
}

#[test]
fn gpu_a_mesh_under_a_moved_group_lands_where_project_puts_it_in_perspective() {
    let (landed, expected) = placed(Projection::perspective(1.0, 0.1, 100.0).unwrap());
    assert_within_a_pixel(landed, expected);
}

#[test]
fn gpu_a_mesh_under_a_moved_group_lands_where_project_puts_it_in_orthographic() {
    let (landed, expected) = placed(Projection::orthographic(6.0, 0.1, 100.0).unwrap());
    assert_within_a_pixel(landed, expected);
}

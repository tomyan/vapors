//! Mesh uniform slots: more mesh nodes than the first buffer holds all
//! draw where they stand, including after the buffer is reallocated.

mod support;

use support::{Fixture, RED, at, render, to_pixel};
use vapors_scene3d::{Projection, Transform, Vec3, View, project, view_projection};

/// Mesh `i` of a row across the 16-unit-wide view: x from -7.2 to 7.2.
#[allow(clippy::cast_precision_loss)]
fn place(i: usize) -> Vec3 {
    Vec3::new((i as f32 - 8.0) * 0.9, 0.0, 0.0)
}

#[test]
fn gpu_meshes_added_past_the_first_slot_buffer_all_draw_where_they_stand() {
    // Given: 16 squares rendered, filling the first buffer, then a 17th
    let gpu = vapors_gpu_harness::gpu();
    let fixture = Fixture::new();
    let projection = Projection::orthographic(12.0, 0.1, 100.0).unwrap();
    let eye = Vec3::new(0.0, 0.0, 5.0);
    fixture.camera(projection, eye);
    let square = fixture.square(0.3);
    let root = fixture.root();
    let add = |i| {
        fixture.mesh(
            root,
            &square,
            RED,
            Transform::from_translation(place(i)).unwrap(),
        )
    };
    (0..16).for_each(|i| _ = add(i));
    let mut renderer = fixture.renderer(&gpu);
    render(&mut renderer, &gpu).expect("renders");
    add(16);

    // When
    let pixels = render(&mut renderer, &gpu).expect("renders");

    // Then
    let view = View::look_at(eye, Vec3::ZERO, Vec3::Y).unwrap();
    let matrix = view_projection(&projection, &view, 160.0 / 120.0).unwrap();
    for i in 0..17 {
        let [x, y] = to_pixel(project(matrix, place(i)).unwrap());
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let pixel = at(&pixels, x as u32, y as u32);
        assert_eq!(pixel, RED, "mesh {i} at pixel ({x}, {y})");
    }
}

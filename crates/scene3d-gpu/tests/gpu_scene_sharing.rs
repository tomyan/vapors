//! Several renderers over one `Meshes`: each uploads the geometry it
//! has not uploaded itself, so two panels on one scene, or a renderer
//! made after another has rendered (a remount), draw every mesh.

mod support;

use support::{Fixture, RED, centroid, render};
use vapors_scene3d::{Projection, Transform, Vec3};

/// A camera and one red square at the centre.
fn one_square() -> Fixture {
    let fixture = Fixture::new();
    let projection = Projection::orthographic(6.0, 0.1, 100.0).unwrap();
    fixture.camera(projection, Vec3::new(0.0, 0.0, 5.0));
    let square = fixture.square(0.5);
    fixture.mesh(fixture.root(), square, RED, Transform::IDENTITY);
    fixture
}

#[test]
fn gpu_two_renderers_over_one_meshes_both_draw_the_mesh() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let fixture = one_square();
    let mut first = fixture.renderer(&gpu);
    let mut second = fixture.renderer(&gpu);

    // When
    let first_pixels = render(&mut first, &gpu).expect("the first renders");
    let second_pixels = render(&mut second, &gpu).expect("the second renders");

    // Then
    assert!(centroid(&first_pixels, RED).1 > 20, "the first draws it");
    assert!(centroid(&second_pixels, RED).1 > 20, "the second draws it");
}

#[test]
fn gpu_a_renderer_made_after_another_has_rendered_draws_the_mesh() {
    // Given: the first renderer has rendered and is gone, as on unmount
    let gpu = vapors_gpu_harness::gpu();
    let fixture = one_square();
    let mut first = fixture.renderer(&gpu);
    render(&mut first, &gpu).expect("the first renders");
    drop(first);

    // When
    let mut remounted = fixture.renderer(&gpu);
    let pixels = render(&mut remounted, &gpu).expect("the remounted one renders");

    // Then
    assert!(centroid(&pixels, RED).1 > 20, "it draws the square");
}

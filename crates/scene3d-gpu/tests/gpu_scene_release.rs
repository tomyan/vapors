//! Geometry release (`docs/plans/3d-2.md` row 5, decision (B)): a piece
//! is held while any `GeometryHandle` to it lives, the application's or
//! a mesh node's; once the last is dropped, the next render releases it
//! from the `Meshes` and drops this renderer's buffers for it.

mod support;

use support::{Fixture, GREEN, RED, render};
use vapors_scene3d::{Projection, Transform, Vec3};
use vapors_scene3d_gpu::{RendererLog, Resource};

/// The pieces of geometry the renderer holds on the GPU.
fn on_gpu(log: &RendererLog) -> u64 {
    log.made(Resource::Geometry) - log.freed(Resource::Geometry)
}

#[test]
fn gpu_a_removed_meshs_piece_no_handle_claims_is_released_at_the_next_render() {
    // Given: two meshes of their own pieces, drawn; the application
    // keeps neither handle
    let gpu = vapors_gpu_harness::gpu();
    let fixture = Fixture::new();
    let projection = Projection::orthographic(6.0, 0.1, 100.0).unwrap();
    fixture.camera(projection, Vec3::new(0.0, 0.0, 5.0));
    let small = fixture.square(0.5);
    let small_id = small.id();
    let gone = fixture.mesh(fixture.root(), &small, RED, Transform::IDENTITY);
    fixture.mesh(
        fixture.root(),
        &fixture.square(1.0),
        GREEN,
        Transform::IDENTITY,
    );
    drop(small);
    let mut renderer = fixture.renderer(&gpu);
    let log = renderer.log();
    render(&mut renderer, &gpu).expect("renders");
    render(&mut renderer, &gpu).expect("renders");
    assert_eq!(on_gpu(&log), 2);

    // When
    fixture.scene.borrow_mut().remove(gone).unwrap();
    render(&mut renderer, &gpu).expect("renders");

    // Then
    assert!(!fixture.meshes.borrow().holds(small_id), "released");
    assert_eq!(on_gpu(&log), 1);
}

#[test]
fn gpu_a_piece_the_application_still_holds_stays_after_its_mesh_is_removed() {
    // Given: the application keeps its handle, as a page that shows the
    // mesh again under an `{if}` does
    let gpu = vapors_gpu_harness::gpu();
    let fixture = Fixture::new();
    let projection = Projection::orthographic(6.0, 0.1, 100.0).unwrap();
    fixture.camera(projection, Vec3::new(0.0, 0.0, 5.0));
    let square = fixture.square(0.5);
    let node = fixture.mesh(fixture.root(), &square, RED, Transform::IDENTITY);
    let mut renderer = fixture.renderer(&gpu);
    render(&mut renderer, &gpu).expect("renders");

    // When
    fixture.scene.borrow_mut().remove(node).unwrap();
    render(&mut renderer, &gpu).expect("renders");
    fixture.mesh(fixture.root(), &square, RED, Transform::IDENTITY);
    let shown = render(&mut renderer, &gpu);

    // Then
    assert!(fixture.meshes.borrow().holds(square.id()));
    assert!(shown.is_ok(), "{shown:?}");
}

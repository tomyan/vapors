//! What a lit mesh is refused for, each naming the fix.

mod support;

use support::lit::{box_mesh, shining, sun};
use support::{Fixture, RED, render};
use vapors_scene3d::{Ambient, Colour, Projection, Shading, Transform, Vec3};

fn fixture() -> Fixture {
    let fixture = Fixture::new();
    let projection = Projection::orthographic(6.0, 0.1, 100.0).unwrap();
    fixture.camera(projection, Vec3::new(0.0, 0.0, 5.0));
    fixture
}

/// The refusal of the first frame, which must name each of `needles`.
fn refused(fixture: &Fixture, needles: &[&str]) {
    let gpu = vapors_gpu_harness::gpu();
    let mut renderer = fixture.renderer(&gpu);
    let message = render(&mut renderer, &gpu)
        .expect_err("refused")
        .to_string();
    for needle in needles {
        assert!(message.contains(needle), "no {needle:?} in: {message}");
    }
}

#[test]
fn gpu_a_lit_mesh_without_normals_is_refused_naming_node_and_geometry() {
    // a plain square, set lit, and a light
    let fixture = fixture();
    let square = fixture.square(0.5);
    let node = fixture.mesh(fixture.root(), &square, RED, Transform::IDENTITY);
    let lit = fixture.scene.borrow_mut().set_shading(node, Shading::Lit);
    lit.unwrap();
    sun(&fixture, shining(-Vec3::Z));

    // When / Then
    refused(&fixture, &[&node.to_string(), "geometry", "lit_geometry"]);
}

#[test]
fn gpu_a_lit_mesh_with_no_light_and_no_ambient_is_refused_naming_the_fix() {
    // Given
    let fixture = fixture();
    let node = box_mesh(&fixture, RED, Transform::IDENTITY, Shading::Lit);

    // When / Then
    refused(&fixture, &[&node.to_string(), "<Light>", "ambient"]);
}

#[test]
fn gpu_ambient_alone_lights_a_lit_mesh() {
    // Given
    let fixture = fixture();
    box_mesh(&fixture, RED, Transform::IDENTITY, Shading::Lit);
    let ambient = Ambient::new(Colour::srgb(1.0, 1.0, 1.0).unwrap(), 0.5).unwrap();
    fixture.scene.borrow_mut().set_ambient(ambient);
    let gpu = vapors_gpu_harness::gpu();
    let mut renderer = fixture.renderer(&gpu);

    // When / Then
    render(&mut renderer, &gpu).expect("renders");
}

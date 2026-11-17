//! GPU tier: application materials (one-scene D5 (b), R2). A mesh
//! drawn with a material takes its colour from the material's WGSL,
//! which sees the fragment's world position, screen position, the
//! target's size, the mesh's colour and the vertex shade; a material
//! given after the renderer mounted, or another `Meshes`' material, is
//! refused naming the node.

mod support;

use support::{Fixture, SIZE, at, render};
use vapors_scene3d::{Material, MaterialId, Meshes, NodeId, Projection, Transform, Vec3};

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

/// A material whose WGSL body returns `body`'s value, given `at`.
fn returning(label: &str, body: &str) -> Material {
    Material {
        label: label.into(),
        wgsl: format!("fn material(at: MaterialInput) -> vec4<f32> {{ {body} }}"),
    }
}

/// A fixture looking straight down -Z at a 4-wide square at the origin
/// that fills the backing's height, in `rgba`, and the square's node.
fn square_in_view(rgba: [u8; 4]) -> (Fixture, NodeId) {
    let fixture = Fixture::new();
    fixture.camera(
        Projection::orthographic(4.0, 0.1, 100.0).unwrap(),
        Vec3::new(0.0, 0.0, 5.0),
    );
    let square = fixture.square(2.0);
    let node = fixture.mesh(fixture.root(), &square, rgba, Transform::IDENTITY);
    (fixture, node)
}

/// `material` given to the fixture's `Meshes` and set on `node`.
fn with_material(fixture: &Fixture, node: NodeId, material: Material) -> MaterialId {
    let id = fixture.meshes.borrow_mut().material(material);
    fixture
        .scene
        .borrow_mut()
        .set_material(node, Some(id))
        .unwrap();
    id
}

#[test]
fn gpu_a_material_colours_a_mesh_by_world_position() {
    // Given: red where x < 0, blue elsewhere
    let gpu = vapors_gpu_harness::gpu();
    let (fixture, node) = square_in_view([0, 255, 0, 255]);
    let body =
        "if at.world.x < 0.0 { return vec4(1.0, 0.0, 0.0, 1.0); } return vec4(0.0, 0.0, 1.0, 1.0);";
    with_material(&fixture, node, returning("halves", body));
    let mut renderer = fixture.renderer(&gpu);

    // When
    let pixels = render(&mut renderer, &gpu).expect("renders");

    // Then: left of the centre red, right of it blue
    let (x, y) = (SIZE.width / 2, SIZE.height / 2);
    assert_eq!(
        (at(&pixels, x - 10, y), at(&pixels, x + 10, y)),
        (RED, BLUE)
    );
}

#[test]
fn gpu_a_material_returning_colour_times_shade_draws_as_the_colour_pipeline() {
    // Given: the same square, once by colour and once by a material
    // that gives back what the colour pipeline draws
    let gpu = vapors_gpu_harness::gpu();
    let colour = [200, 120, 60, 255];
    let (plain, _) = square_in_view(colour);
    let (materialled, node) = square_in_view(colour);
    let body = "return vec4(at.colour.rgb * at.shade, at.colour.a);";
    with_material(&materialled, node, returning("colour", body));

    // When
    let by_colour = render(&mut plain.renderer(&gpu), &gpu).expect("renders");
    let by_material = render(&mut materialled.renderer(&gpu), &gpu).expect("renders");

    // Then
    assert_eq!(by_material, by_colour);
}

#[test]
fn gpu_a_material_sees_the_fragments_screen_position_and_the_targets_size() {
    // Given: red where the fragment is left of the target's middle
    let gpu = vapors_gpu_harness::gpu();
    let (fixture, node) = square_in_view([0, 255, 0, 255]);
    let body = "if at.screen.x < at.size.x * 0.5 { return vec4(1.0, 0.0, 0.0, 1.0); } return vec4(0.0, 0.0, 1.0, 1.0);";
    with_material(&fixture, node, returning("screen", body));
    let mut renderer = fixture.renderer(&gpu);

    // When
    let pixels = render(&mut renderer, &gpu).expect("renders");

    // Then: the split is at the backing's middle column
    let (x, y) = (SIZE.width / 2, SIZE.height / 2);
    assert_eq!((at(&pixels, x - 1, y), at(&pixels, x, y)), (RED, BLUE));
}

#[test]
fn gpu_a_material_given_after_the_renderer_mounted_is_refused_naming_the_node() {
    // Given: the renderer mounted, then the material given
    let gpu = vapors_gpu_harness::gpu();
    let (fixture, node) = square_in_view(RED);
    let mut renderer = fixture.renderer(&gpu);
    let material = with_material(&fixture, node, returning("late", "return at.colour;"));

    // When
    let refused = render(&mut renderer, &gpu).expect_err("refused");

    // Then
    assert_eq!(
        refused.to_string(),
        format!(
            "{node} names {material}, and this renderer built 0 materials when it mounted: give every material to the Meshes before mounting the renderer"
        )
    );
}

#[test]
fn gpu_another_meshes_material_is_refused_naming_the_node() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let (fixture, node) = square_in_view(RED);
    let foreign = Meshes::new().material(returning("foreign", "return at.colour;"));
    fixture
        .scene
        .borrow_mut()
        .set_material(node, Some(foreign))
        .unwrap();
    let mut renderer = fixture.renderer(&gpu);

    // When
    let refused = render(&mut renderer, &gpu).expect_err("refused");

    // Then
    assert!(
        refused.to_string().starts_with(&format!(
            "{node} names {foreign}, which another Meshes issued"
        )),
        "{refused}"
    );
}

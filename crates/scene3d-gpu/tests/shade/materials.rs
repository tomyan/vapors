//! GPU tier: application materials (one-scene D5 (b), R2). A mesh
//! drawn with a material takes its colour from the material's WGSL,
//! which sees the fragment's world position, its place in the scene's
//! viewport and the viewport's size, the mesh's colour and the vertex
//! shade; meshes on two materials and on their colour each draw with
//! their own; and draws keep the tree's order, so of two at one depth
//! the first in the tree wins.

use super::support::materials::{
    BLUE, RED, flat, looking_down, returning, square_in_view, with_material,
};
use super::support::{SIZE, at, render};
use vapors_scene3d::{Transform, Vec3};

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
fn gpu_a_material_sees_its_place_in_the_scenes_viewport_and_its_size() {
    // Given: red where the fragment is left of the viewport's middle
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
fn gpu_meshes_on_two_materials_and_on_their_colour_each_draw_their_own() {
    // Given: three unit squares across the view, x -1.5, 0 and 1.5: on
    // material A, material B, and the third in its colour
    let gpu = vapors_gpu_harness::gpu();
    let fixture = looking_down();
    let square = fixture.square(0.5);
    let green = [0, 255, 0, 255];
    let place = |x| Transform::from_translation(Vec3::new(x, 0.0, 0.0)).unwrap();
    let nodes = [-1.5, 0.0, 1.5].map(|x| fixture.mesh(fixture.root(), &square, BLUE, place(x)));
    with_material(&fixture, nodes[0], flat("a", RED));
    with_material(&fixture, nodes[1], flat("b", green));
    let mut renderer = fixture.renderer(&gpu);

    // When
    let pixels = render(&mut renderer, &gpu).expect("renders");

    // Then: 30 pixels a unit from the centre
    let (x, y) = (SIZE.width / 2, SIZE.height / 2);
    let drawn = [x - 45, x, x + 45].map(|x| at(&pixels, x, y));
    assert_eq!(drawn, [RED, green, BLUE]);
}

#[test]
fn gpu_of_two_meshes_at_one_depth_the_first_in_the_tree_wins_whatever_draws_them() {
    // Given: a square on a material, then the same square in its colour,
    // at one depth
    let gpu = vapors_gpu_harness::gpu();
    let fixture = looking_down();
    let square = fixture.square(1.0);
    let first = fixture.mesh(fixture.root(), &square, BLUE, Transform::IDENTITY);
    fixture.mesh(fixture.root(), &square, BLUE, Transform::IDENTITY);
    with_material(&fixture, first, flat("first", RED));
    let mut renderer = fixture.renderer(&gpu);

    // When
    let pixels = render(&mut renderer, &gpu).expect("renders");

    // Then: the first's, as drawn in tree order (depth `Less`)
    assert_eq!(at(&pixels, SIZE.width / 2, SIZE.height / 2), RED);
}

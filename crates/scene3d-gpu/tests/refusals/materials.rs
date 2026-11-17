//! GPU tier: a material the renderer did not build at mount, or another
//! `Meshes`' material, is refused naming the node (one-scene R2).

use super::support::materials::{RED, returning, set, square_in_view, with_material};
use super::support::render;
use vapors_scene3d::Meshes;

#[test]
fn gpu_a_material_given_after_the_renderer_mounted_is_refused_naming_the_node_and_label() {
    // Given: the renderer mounted, then the material given
    let gpu = vapors_gpu_harness::gpu();
    let (fixture, node) = square_in_view(RED);
    let mut renderer = fixture.renderer(&gpu);
    let material = with_material(&fixture, node, returning("late", "return at.colour;"));

    // When
    let refused = render(&mut renderer, &gpu).expect_err("refused");

    // Then: the id names the label
    assert_eq!(
        refused.to_string(),
        format!(
            "{node} names {material}, and this renderer built 0 materials when it mounted: give every material to the Meshes before mounting the renderer"
        )
    );
    assert!(refused.to_string().contains("(`late`)"), "{refused}");
}

#[test]
fn gpu_another_meshes_material_is_refused_naming_the_node_and_label() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let (fixture, node) = square_in_view(RED);
    let foreign = Meshes::new()
        .material(returning("foreign", "return at.colour;"))
        .expect("held");
    set(&fixture, node, foreign);
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
    assert!(refused.to_string().contains("(`foreign`)"), "{refused}");
}

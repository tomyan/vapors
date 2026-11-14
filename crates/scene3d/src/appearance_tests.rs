use crate::test_support::mesh;
use crate::{Colour, NodeId, NodeKind, Scene, StoreError, Transform};

/// root > (a > a2x), root > b; a2x and b are meshes, updated once.
struct Tree {
    scene: Scene,
    a: NodeId,
    a2x: NodeId,
    b: NodeId,
}

fn tree() -> Tree {
    let mut scene = Scene::new();
    let root = scene.root();
    let a = scene
        .add(root, NodeKind::Group, Transform::IDENTITY)
        .unwrap();
    let a2x = scene.add(a, mesh(), Transform::IDENTITY).unwrap();
    let b = scene.add(root, mesh(), Transform::IDENTITY).unwrap();
    scene.update();
    Tree { scene, a, a2x, b }
}

/// The colour `test_support::mesh` gives.
fn grey() -> Colour {
    Colour::srgb(0.5, 0.5, 0.5).unwrap()
}

fn blue() -> Colour {
    Colour::srgb(0.0, 0.0, 1.0).unwrap()
}

#[test]
fn setting_a_mesh_colour_advances_its_world_version_alone_of_the_meshes() {
    // Given: a2x and b are meshes
    let mut tree = tree();
    let before = [tree.a2x, tree.b].map(|id| tree.scene.world_version(id).unwrap());

    // When
    tree.scene.set_appearance(tree.a2x, blue(), true).unwrap();

    // Then
    let after = [tree.a2x, tree.b].map(|id| tree.scene.world_version(id).unwrap());
    assert!(after[0] > before[0], "a2x: {before:?} then {after:?}");
    assert_eq!(after[1], before[1], "b: {before:?} then {after:?}");
}

#[test]
fn setting_a_mesh_colour_changes_its_kind_and_recomputes_no_world_matrix() {
    // Given
    let mut tree = tree();

    // When
    tree.scene.set_appearance(tree.b, blue(), true).unwrap();

    // Then
    let colour = match tree.scene.kind(tree.b).unwrap() {
        NodeKind::Mesh { colour, .. } => *colour,
        other => panic!("b is a mesh, found {other:?}"),
    };
    assert_eq!(colour, blue());
    assert_eq!(tree.scene.update(), []);
}

#[test]
fn setting_the_colour_of_a_node_that_is_not_a_mesh_is_refused() {
    // Given: a is a group
    let mut tree = tree();

    // When
    let refused = tree.scene.set_appearance(tree.a, blue(), true);

    // Then
    assert_eq!(refused, Err(StoreError::NotMesh(tree.a)));
    assert!(
        refused.unwrap_err().to_string().contains("set_appearance"),
        "names the call"
    );
}

#[test]
fn black_is_the_colour_with_every_channel_zero() {
    assert_eq!(Colour::BLACK, Colour::srgb(0.0, 0.0, 0.0).unwrap());
}

#[test]
fn setting_a_mesh_appearance_to_another_colour_advances_its_world_version() {
    // Given
    let mut tree = tree();
    let before = tree.scene.world_version(tree.b).unwrap();

    // When
    let changed = tree.scene.set_appearance(tree.b, blue(), true).unwrap();

    // Then
    assert!(changed);
    assert!(tree.scene.world_version(tree.b).unwrap() > before);
    assert!(
        matches!(tree.scene.kind(tree.b).unwrap(), NodeKind::Mesh { colour, .. } if *colour == blue())
    );
}

#[test]
fn setting_a_mesh_appearance_it_already_has_changes_nothing() {
    // Given: the test meshes are grey and shown
    let mut tree = tree();
    let before = tree.scene.world_version(tree.b).unwrap();

    // When
    let changed = tree.scene.set_appearance(tree.b, grey(), true).unwrap();

    // Then
    assert!(!changed);
    assert_eq!(tree.scene.world_version(tree.b).unwrap(), before);
}

#[test]
fn hiding_a_mesh_keeps_its_world_version_and_shows_it_hidden() {
    // Given
    let mut tree = tree();
    let before = tree.scene.world_version(tree.b).unwrap();

    // When
    let changed = tree.scene.set_appearance(tree.b, grey(), false).unwrap();

    // Then: no uniform to write, one draw fewer
    assert!(changed);
    assert_eq!(tree.scene.world_version(tree.b).unwrap(), before);
    assert_eq!(tree.scene.visible(tree.b), Ok(false));
    assert_eq!(tree.scene.visible(tree.a2x), Ok(true));
}

#[test]
fn setting_the_appearance_of_a_group_or_a_removed_mesh_is_refused() {
    // Given
    let mut tree = tree();
    tree.scene.remove(tree.b).unwrap();

    // When
    let group = tree.scene.set_appearance(tree.a, blue(), true);
    let removed = tree.scene.set_appearance(tree.b, blue(), true);

    // Then
    assert_eq!(group, Err(StoreError::NotMesh(tree.a)));
    assert_eq!(removed, Err(StoreError::Stale(tree.b)));
    assert_eq!(tree.scene.visible(tree.b), Err(StoreError::Stale(tree.b)));
}

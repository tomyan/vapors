use super::*;
use crate::{Colour, NodeKind, Scene, Transform};

fn triangle(meshes: &mut Meshes) -> GeometryHandle {
    meshes
        .geometry(
            vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            vec![0, 1, 2],
            vec![1.0; 3],
        )
        .unwrap()
}

#[test]
fn a_piece_is_held_while_any_handle_to_it_lives() {
    // Given: the first handle dropped, a clone kept
    let mut meshes = Meshes::new();
    let first = triangle(&mut meshes);
    let kept = first.clone();
    drop(first);

    // When
    let released = meshes.release_unclaimed();

    // Then
    assert_eq!(released, []);
    assert!(meshes.holds(kept.id()));
}

#[test]
fn a_piece_whose_every_handle_is_dropped_is_released_at_the_next_sweep() {
    // Given
    let mut meshes = Meshes::new();
    let handle = triangle(&mut meshes);
    let id = handle.id();
    drop(handle);

    // When
    let released = meshes.release_unclaimed();

    // Then: a stale id finds nothing
    assert_eq!(released, [id]);
    assert!(!meshes.holds(id));
    assert_eq!(meshes.get(id), None);
    assert!(meshes.issued(id), "issued, then released");
}

#[test]
fn a_mesh_node_naming_a_piece_keeps_it_until_the_node_is_removed() {
    // Given: the application's handle dropped, the node's kept
    let mut meshes = Meshes::new();
    let handle = triangle(&mut meshes);
    let id = handle.id();
    let mut scene = Scene::new();
    let root = scene.root();
    let colour = Colour::BLACK;
    let kind = NodeKind::Mesh {
        geometry: handle,
        colour,
    };
    let node = scene.add(root, kind, Transform::IDENTITY).unwrap();
    assert_eq!(meshes.release_unclaimed(), []);

    // When
    scene.remove(node).unwrap();
    let released = meshes.release_unclaimed();

    // Then
    assert_eq!(released, [id]);
}

#[test]
fn handles_to_one_piece_are_equal_and_name_its_id() {
    // Given
    let mut meshes = Meshes::new();
    let a = triangle(&mut meshes);
    let b = triangle(&mut meshes);

    // Then
    assert_eq!(a.clone(), a);
    assert_ne!(a, b);
    assert_eq!(a.to_string(), a.id().to_string());
}

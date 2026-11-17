//! Materials held by a `Meshes` and named by mesh nodes.

use crate::test_support::mesh;
use crate::{Material, Meshes, NodeKind, Scene, StoreError, Transform};

fn checker() -> Material {
    Material {
        label: "checker".into(),
        wgsl: "fn material(at: MaterialInput) -> vec4<f32> { return at.colour; }".into(),
    }
}

#[test]
fn materials_are_held_in_the_order_given_each_id_its_place() {
    // Given
    let mut meshes = Meshes::new();

    // When
    let (first, second) = (meshes.material(checker()), meshes.material(checker()));

    // Then
    assert_eq!((first.index(), second.index()), (0, 1));
    assert_eq!(meshes.materials().len(), 2);
    assert!(meshes.issued_material(second));
}

#[test]
fn another_meshes_material_is_not_issued_by_this_one() {
    let mut other = Meshes::new();
    let foreign = other.material(checker());
    assert!(!Meshes::new().issued_material(foreign));
}

#[test]
fn a_mesh_takes_a_material_and_gives_it_back_to_its_colour() {
    // Given
    let (mut meshes, mut scene) = (Meshes::new(), Scene::new());
    let material = meshes.material(checker());
    let root = scene.root();
    let node = scene.add(root, mesh(), Transform::IDENTITY).expect("adds");

    // When
    let set = scene.set_material(node, Some(material));
    let same = scene.set_material(node, Some(material));
    let held = scene.material(node);
    let cleared = scene.set_material(node, None);

    // Then
    assert_eq!((set, same, cleared), (Ok(true), Ok(false), Ok(true)));
    assert_eq!(held, Ok(Some(material)));
    assert_eq!(scene.material(node), Ok(None));
}

#[test]
fn a_material_on_a_group_is_refused_naming_the_setter() {
    let (mut meshes, mut scene) = (Meshes::new(), Scene::new());
    let material = meshes.material(checker());
    let root = scene.root();
    let group = scene
        .add(root, NodeKind::Group, Transform::IDENTITY)
        .expect("adds");
    let refused = scene.set_material(group, Some(material));
    assert_eq!(refused, Err(StoreError::NotMesh(group, "set_material")));
    assert!(
        refused
            .unwrap_err()
            .to_string()
            .contains("Scene::set_material")
    );
}

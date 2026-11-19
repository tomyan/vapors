use crate::test_support::mesh;
use crate::{Scene, Shading, StoreError, Transform};

#[test]
fn a_mesh_is_unlit_until_set_and_a_group_cannot_be_set() {
    let mut scene = Scene::new();
    let root = scene.root();
    let node = scene.add(root, mesh(), Transform::IDENTITY).unwrap();
    assert_eq!(scene.shading(node).unwrap(), Shading::Unlit);
    scene.set_shading(node, Shading::Lit).unwrap();
    assert_eq!(scene.shading(node).unwrap(), Shading::Lit);
    assert_eq!(
        scene.set_shading(root, Shading::Lit),
        Err(StoreError::NotMesh(root, "set_shading"))
    );
}

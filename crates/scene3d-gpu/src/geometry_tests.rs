use super::*;
use vapors_scene3d::{Colour, Meshes, NodeKind, Scene, Transform, Vec3};

/// A mesh node in a fresh scene naming a triangle from `meshes`.
fn mesh_in(meshes: &mut Meshes) -> MeshNode {
    let geometry = meshes
        .geometry(
            vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            vec![0, 1, 2],
            vec![1.0; 3],
        )
        .unwrap();
    let colour = Colour::srgb(1.0, 0.0, 0.0).unwrap();
    let mut scene = Scene::new();
    let root = scene.root();
    let id = geometry.id();
    let kind = NodeKind::Mesh { geometry, colour };
    let node = scene.add(root, kind, Transform::IDENTITY).unwrap();
    MeshNode {
        node,
        geometry: id,
        colour,
        visible: true,
    }
}

#[test]
fn issued_geometry_whose_data_is_not_found_is_refused_naming_node_and_geometry() {
    // Given
    let mesh = mesh_in(&mut Meshes::new());

    // When
    let refused =
        GpuGeometry::new(&std::rc::Rc::default()).missing(std::slice::from_ref(&mesh), |_| None);

    // Then
    let error = refused.expect_err("refused");
    assert_eq!(
        error,
        FrameError::GeometryMissing {
            node: mesh.node,
            geometry: mesh.geometry,
        }
    );
    let message = error.to_string();
    assert!(message.contains(&mesh.node.to_string()), "{message}");
    assert!(message.contains(&mesh.geometry.to_string()), "{message}");
}

#[test]
fn geometry_not_yet_uploaded_is_found_once_however_many_nodes_name_it() {
    // Given
    let mut meshes = Meshes::new();
    let mesh = mesh_in(&mut meshes);
    let twice = [mesh_in(&mut Meshes::new()), mesh_in(&mut Meshes::new())].map(|other| MeshNode {
        geometry: mesh.geometry,
        ..other
    });

    // When
    let missing = GpuGeometry::new(&std::rc::Rc::default()).missing(&twice, |id| meshes.get(id));

    // Then
    let missing = missing.expect("found");
    assert_eq!(missing.len(), 1);
    assert_eq!(
        missing.get(&mesh.geometry),
        meshes.get(mesh.geometry).as_ref()
    );
}

#[test]
fn a_mesh_whose_geometry_is_not_uploaded_has_no_buffers() {
    // Given
    let mesh = mesh_in(&mut Meshes::new());

    // When
    let buffers = GpuGeometry::new(&std::rc::Rc::default())
        .get(&mesh)
        .map(|_| ());

    // Then
    assert_eq!(
        buffers,
        Err(FrameError::GeometryMissing {
            node: mesh.node,
            geometry: mesh.geometry,
        })
    );
}

#[test]
fn each_uploaded_vertex_is_the_size_the_scene_checks() {
    // Given
    let mut meshes = Meshes::new();
    let geometry = meshes
        .geometry(
            vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            vec![0, 1, 2],
            vec![0.5; 3],
        )
        .unwrap();

    // When
    let bytes = vertex_bytes(meshes.get(geometry.id()).expect("held"));

    // Then
    assert_eq!(
        bytes.len(),
        3 * vapors_scene3d::VERTEX_BYTES,
        "the upload writes {} bytes for 3 vertices; VERTEX_BYTES is {}: change both together (vapors-scene3d mesh_error.rs and geometry.rs)",
        bytes.len(),
        vapors_scene3d::VERTEX_BYTES
    );
}

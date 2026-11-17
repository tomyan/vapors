use super::*;
use crate::test_support::mesh;
use crate::{Projection, Quat, Transform, Vec3};

fn camera(handle: CameraHandle) -> NodeKind {
    let projection = Projection::perspective(1.0, 0.1, 100.0).unwrap();
    NodeKind::Camera { handle, projection }
}

#[test]
fn the_named_camera_is_active() {
    // Given: two cameras with their own handles
    let mut scene = Scene::new();
    let root = scene.root();
    let (first, second) = (CameraHandle::new(), CameraHandle::new());
    scene.add(root, camera(first), Transform::IDENTITY).unwrap();
    let named = scene
        .add(root, camera(second), Transform::IDENTITY)
        .unwrap();

    // When
    scene.set_camera(Some(second));

    // Then
    assert_eq!(scene.active_camera(), Ok(named));
}

#[test]
fn a_scene_naming_no_camera_is_refused() {
    // Given: a camera, but the root names none
    let mut scene = Scene::new();
    let root = scene.root();
    scene
        .add(root, camera(CameraHandle::new()), Transform::IDENTITY)
        .unwrap();

    // When
    let result = scene.active_camera();

    // Then
    assert_eq!(result, Err(StoreError::NoCamera));
    let message = result.unwrap_err().to_string();
    assert!(message.contains("call Scene::set_camera"), "{message}");
}

#[test]
fn a_handle_no_camera_holds_is_refused() {
    // Given: the root names a camera that was removed
    let mut scene = Scene::new();
    let root = scene.root();
    let handle = CameraHandle::new();
    let node = scene
        .add(root, camera(handle), Transform::IDENTITY)
        .unwrap();
    scene.set_camera(Some(handle));
    scene.remove(node).unwrap();

    // When
    let result = scene.active_camera();

    // Then
    assert_eq!(result, Err(StoreError::CameraUnresolved(handle)));
    let message = result.unwrap_err().to_string();
    assert!(message.contains("which no camera node holds"), "{message}");
}

#[test]
fn a_handle_two_cameras_hold_is_refused_naming_both() {
    // Given
    let mut scene = Scene::new();
    let root = scene.root();
    let handle = CameraHandle::new();
    let first = scene
        .add(root, camera(handle), Transform::IDENTITY)
        .unwrap();
    let second = scene
        .add(root, camera(handle), Transform::IDENTITY)
        .unwrap();
    scene.set_camera(Some(handle));

    // When
    let result = scene.active_camera();

    // Then
    let nodes = vec![first, second];
    assert_eq!(result, Err(StoreError::CameraDuplicate { handle, nodes }));
    let message = result.unwrap_err().to_string();
    assert!(
        message.contains(&format!("cameras {first}, {second}")),
        "{message}"
    );
    assert!(
        message.contains("give each camera its own handle"),
        "{message}"
    );
}

#[test]
fn a_group_or_mesh_never_answers_for_a_camera() {
    // Given: no camera nodes at all
    let mut scene = Scene::new();
    let root = scene.root();
    scene.add(root, mesh(), Transform::IDENTITY).unwrap();
    let handle = CameraHandle::new();
    scene.set_camera(Some(handle));

    // When / Then
    assert_eq!(
        scene.active_camera(),
        Err(StoreError::CameraUnresolved(handle))
    );
}

/// A scene whose active camera sits under a group placed by `group`.
fn camera_under(group: Transform) -> (Scene, NodeId) {
    let mut scene = Scene::new();
    let handle = CameraHandle::new();
    let root = scene.root();
    let parent = scene.add(root, NodeKind::Group, group).unwrap();
    let node = scene
        .add(parent, camera(handle), Transform::IDENTITY)
        .unwrap();
    scene.set_camera(Some(handle));
    scene.update();
    (scene, node)
}

#[test]
fn a_camera_under_a_scaling_or_mirroring_group_is_refused_naming_it() {
    for (scale, mirrored) in [(Vec3::splat(2.0), false), (Vec3::new(1.0, -1.0, 1.0), true)] {
        // Given
        let group = Transform::new(Vec3::ZERO, Quat::IDENTITY, scale).unwrap();
        let (scene, node) = camera_under(group);

        // When
        let refused = scene.view_projection(1.0).expect_err("refused");

        // Then
        let CameraError::NotRigid {
            node: named,
            mirrored: found,
            ..
        } = refused
        else {
            panic!("{refused:?}");
        };
        assert_eq!((named, found), (node, mirrored));
    }
}

#[test]
fn a_camera_under_a_turned_and_moved_group_is_held() {
    let turned = Quat::from_rotation_y(0.7) * Quat::from_rotation_x(-0.4);
    let group = Transform::new(Vec3::new(3.0, -2.0, 9.0), turned, Vec3::ONE).unwrap();
    assert!(camera_under(group).0.view_projection(1.5).is_ok());
}

#[test]
fn a_cameras_lens_is_set_and_a_mesh_is_refused() {
    // Given
    let (mut scene, node) = camera_under(Transform::IDENTITY);
    let wide = Projection::orthographic_shorter(8.0, 0.0, 10.0).unwrap();

    // When
    let changed = (
        scene.set_projection(node, wide),
        scene.set_projection(node, wide),
    );

    // Then
    assert_eq!(changed, (Ok(true), Ok(false)));
    let NodeKind::Camera { projection, .. } = scene.kind(node).unwrap() else {
        panic!("a camera");
    };
    assert_eq!(*projection, wide);
    let root = scene.root();
    assert_eq!(
        scene.set_projection(root, wide),
        Err(StoreError::NotCamera(root, "set_projection"))
    );
}

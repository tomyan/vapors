use super::*;
use crate::{Projection, Transform};

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
    scene
        .add(root, NodeKind::Mesh, Transform::IDENTITY)
        .unwrap();
    let handle = CameraHandle::new();
    scene.set_camera(Some(handle));

    // When / Then
    assert_eq!(
        scene.active_camera(),
        Err(StoreError::CameraUnresolved(handle))
    );
}

//! What the components refuse at mount, naming the fix.

mod support;

use std::rc::Rc;
use support::Rig;
use vapors_scene3d::CameraHandle;
use vapors_scene3d_components::{Camera, CameraProps, Mesh, MeshProps, Scene, SceneProps};

#[test]
#[should_panic(expected = "<Mesh> is placed in an element that is not a scene node")]
fn a_mesh_outside_a_scene_is_refused() {
    // Given
    let rig = Rig::new();
    let body = rig.doc.borrow().body();

    let props = MeshProps {
        geometry: Some(support::square(&rig.meshes)),
        ..MeshProps::default()
    };

    // When
    Mesh::mount(&rig.doc, body, props);
}

#[test]
#[should_panic(expected = "<Mesh> has no geometry")]
fn a_mesh_without_geometry_is_refused() {
    // Given: the scene's anchor, after the surface
    let rig = Rig::new();
    let anchor = rig.page.roots()[1];

    // When
    Mesh::mount(&rig.doc, anchor, MeshProps::default());
}

#[test]
#[should_panic(expected = "<Mesh>: ")]
fn a_colour_set_on_a_removed_mesh_is_refused() {
    // Given: the mesh's node removed behind the component's back
    let rig = Rig::new();
    let mut scene = rig.scene.borrow_mut();
    let group = scene.children(scene.root()).unwrap()[1];
    let mesh = scene.children(group).unwrap()[0];
    scene.remove(mesh).unwrap();
    drop(scene);

    // When
    rig.signals.left_colour.set(support::red());
}

#[test]
#[should_panic(expected = "<Scene> needs scene={Some(..)} and surface={Some(..)}")]
fn a_scene_without_its_surface_is_refused() {
    // Given
    let rig = Rig::new();
    let body = rig.doc.borrow().body();
    let props = SceneProps {
        scene: Some(Rc::clone(&rig.scene)),
        ..SceneProps::default()
    };

    // When
    Scene::mount(&rig.doc, body, props);
}

#[test]
#[should_panic(expected = "<Camera> needs handle={Some(..)} and projection={Some(..)}")]
fn a_camera_without_a_projection_is_refused() {
    // Given: the scene's anchor, after the surface
    let rig = Rig::new();
    let anchor = rig.page.roots()[1];
    let props = CameraProps {
        handle: Some(CameraHandle::new()),
        ..CameraProps::default()
    };

    // When
    Camera::mount(&rig.doc, anchor, props);
}

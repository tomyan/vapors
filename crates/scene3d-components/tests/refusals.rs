//! What the components refuse at mount, naming the fix.

mod support;

use std::rc::Rc;
use support::Rig;
use vapors_scene3d::CameraHandle;
use vapors_scene3d_components::{
    Camera, CameraProps, Group, GroupProps, Mesh, MeshProps, Scene, SceneProps,
};

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
fn a_colour_for_a_removed_mesh_refuses_the_update_naming_it() {
    // Given: the mesh's node removed behind the component's back
    let rig = Rig::new();
    let mut scene = rig.scene.borrow_mut();
    let group = scene.children(scene.root()).unwrap()[1];
    let mesh = scene.children(group).unwrap()[0];
    scene.remove(mesh).unwrap();
    drop(scene);

    // When
    rig.signals.blue.set(true);
    let viewport = vapors_elements::Viewport {
        width: 64.0,
        height: 64.0,
        scale: 1.0,
    };
    let Err(errors) = rig.doc.borrow_mut().update(viewport) else {
        panic!("the update succeeded; a refusal was expected")
    };

    // Then
    let refused = errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        refused.contains("<mesh class=\"left blue\">") && refused.contains("was removed"),
        "{refused}"
    );
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

#[test]
fn markup_inside_a_group_is_refused_naming_its_template_site() {
    // Given
    let rig = Rig::new();
    let body = rig.doc.borrow().body();
    let props = support::markup_in_group::MarkupInGroupProps {
        scene: Some(Rc::clone(&rig.scene)),
        surface: Some(rig.surface.clone()),
    };

    // When
    let mount = || support::markup_in_group::MarkupInGroup::mount(&rig.doc, body, props);
    let Err(refusal) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(mount)) else {
        panic!("markup inside a group was mounted, not refused");
    };

    // Then
    let message = refusal.downcast_ref::<String>().expect("a formatted panic");
    assert!(message.contains("<Group> holds <p> at "), "{message}");
    assert!(message.contains("markup_in_group.wisp:10:"), "{message}");
}

/// A key a template `<div>` would have, not a group's.
static DIV_KEY: vapors_dom::StyleKey = vapors_dom::StyleKey {
    site: "refusals.rs:1:1",
    scope: None,
    tag: "div",
    classes: &[],
    toggles: &[],
    attributes: &[],
    style: None,
};

#[test]
#[should_panic(
    expected = "<Group> was passed the key of a <div> (refusals.rs:1:1): its anchor's tag is `group`"
)]
fn a_group_given_a_key_with_another_tag_is_refused_naming_the_keys_site() {
    // Given: the scene's anchor, after the surface
    let rig = Rig::new();
    let anchor = rig.page.roots()[1];
    let props = GroupProps {
        anchor: Some(&DIV_KEY),
        ..GroupProps::default()
    };

    // When
    Group::mount(&rig.doc, anchor, props);
}

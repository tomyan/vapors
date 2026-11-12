//! What the components refuse at mount, naming the fix.

mod support;

use std::rc::Rc;
use support::Rig;
use vapors_scene3d_components::{Group, GroupProps, Scene, SceneProps};

#[test]
#[should_panic(expected = "<Group> is placed in an element that is not a scene node")]
fn a_group_outside_a_scene_is_refused() {
    // Given
    let rig = Rig::new();
    let body = rig.doc.borrow().body();

    // When
    Group::mount(&rig.doc, body, GroupProps::default());
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

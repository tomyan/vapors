//! The scene property set on the components' anchors
//! (`docs/plans/css-for-3d.md`, C2): each anchor's used style is a
//! `SceneUsed`; a declaration its tag does not read is refused at the
//! update, naming the rule and the anchor's site. An unclassed mesh's
//! key is its component's own, which the compiler never emits, so this
//! run-time check is the only one that sees it.

mod support;

use support::Rig;
use support::keyed::KeyedRig;
use vapors_elements::{ElementError, Viewport};

const VIEWPORT: Viewport = Viewport {
    width: 64.0,
    height: 64.0,
    scale: 1.0,
};

#[test]
fn a_box_longhand_reaching_an_unclassed_mesh_is_refused_naming_the_rule_and_its_site() {
    // Given: the scene page's two meshes, neither with a class
    let rig = Rig::new();
    rig.signals.shown.set(true);
    let mut doc = rig.doc.borrow_mut();
    doc.add_stylesheet("scene.css", "mesh { width: 10px; }")
        .unwrap();

    // When
    let Err(errors) = doc.update(VIEWPORT) else {
        panic!("the update succeeded; a refusal was expected")
    };

    // Then
    let refused: Vec<String> = errors.iter().map(ElementError::to_string).collect();
    let refused = refused.join("\n");
    assert!(
        refused.contains("scene.css:1:")
            && refused.contains("`width: 10px`")
            && refused.contains("own_keys.rs")
            && refused.contains("<Mesh geometry>"),
        "{refused}"
    );
}

#[test]
fn a_class_toggle_on_a_mesh_restyles_the_one_anchor_it_reaches() {
    // Given
    let rig = KeyedRig::new();
    rig.update();

    // When
    rig.braking.set(true);
    let counts = rig.update();

    // Then
    assert_eq!(counts.elements_restyled, 1, "{counts:?}");
}

#[test]
fn each_anchor_has_a_scene_used_and_the_braking_mesh_takes_its_colour() {
    // Given
    let rig = KeyedRig::new();
    rig.braking.set(true);

    // When
    rig.update();

    // Then: `.braking` is #00ff00
    let doc = rig.doc.borrow();
    let anchors = rig.anchors();
    assert!(anchors.iter().all(|&a| doc.scene_used(a).is_some()));
    let colour = doc.scene_used(anchors[2]).unwrap().colour;
    assert_eq!(colour.to_rgba8(), [0, 255, 0, 255]);
}

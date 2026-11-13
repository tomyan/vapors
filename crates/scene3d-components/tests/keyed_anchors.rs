//! Each scene component's anchor is a keyed element with the scene tag
//! (`docs/plans/css-for-3d.md`, C1): the key the compiler emitted for a
//! tag carrying `class` or `class:`, or the component's own, so anchors
//! share computed styles as template elements do. No computed value
//! reaches a node yet: that is C2 and C3.

mod support;

use support::Rig;
use support::keyed::KeyedRig;
use vapors_dom::Dom;
use vapors_scene3d::{Colour, NodeKind};

#[test]
fn each_anchor_is_created_from_a_key_with_its_scene_tag() {
    // When
    let rig = KeyedRig::new();

    // Then
    let doc = rig.doc.borrow();
    let tags: Vec<&str> = (rig.anchors().iter())
        .map(|anchor| doc.style_key(*anchor).expect("a keyed anchor").tag)
        .collect();
    assert_eq!(tags, ["scene", "group", "mesh", "mesh", "mesh"]);
}

#[test]
fn a_class_directive_on_a_mesh_toggles_its_anchors_class() {
    // Given
    let rig = KeyedRig::new();
    let mesh = rig.anchors()[2];
    let class = |rig: &KeyedRig| {
        let doc = rig.doc.borrow();
        doc.current_attribute(mesh, "class").map(str::to_owned)
    };

    // When
    let before = class(&rig);
    rig.braking.set(true);

    // Then
    assert_eq!(before.as_deref(), Some("body"));
    assert_eq!(class(&rig).as_deref(), Some("body braking"));
}

#[test]
fn two_meshes_of_one_key_share_one_computed_style_and_no_anchor_is_unkeyed() {
    // Given: the two wheels, one template site in a {for}
    let rig = KeyedRig::new();

    // When
    let counts = rig.update();

    // Then: the root and the body are the only unkeyed elements
    assert_eq!(counts.styles_shared, 1, "{counts:?}");
    assert_eq!(counts.styles_unkeyed, 2, "{counts:?}");
}

#[test]
fn meshes_without_classes_share_their_components_own_key() {
    // Given: the scene page's two meshes in one group, neither with a class
    let rig = Rig::new();
    rig.signals.shown.set(true);
    let viewport = vapors_elements::Viewport {
        width: 64.0,
        height: 64.0,
        scale: 1.0,
    };

    // When
    rig.doc.borrow_mut().update(viewport).unwrap();

    // Then
    let counts = rig.doc.borrow().update_counts();
    assert_eq!(counts.styles_shared, 1, "{counts:?}");
}

#[test]
fn a_mesh_keeps_its_colour_prop_whatever_its_anchors_computed_colour() {
    // Given: the body's colour inherits to every anchor and `.body`
    // sets its own; C1 reads neither, so no page colour tints a mesh
    let rig = KeyedRig::new();
    rig.braking.set(true);

    // When
    rig.update();

    // Then
    let scene = rig.scene.borrow();
    let group = scene.children(scene.root()).unwrap()[0];
    for mesh in scene.children(group).unwrap() {
        let kind = scene.kind(*mesh).unwrap();
        assert!(
            matches!(kind, NodeKind::Mesh { colour, .. } if *colour == Colour::BLACK),
            "{kind:?}"
        );
    }
}

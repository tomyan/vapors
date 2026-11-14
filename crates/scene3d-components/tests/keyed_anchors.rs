//! Each scene component's anchor is a keyed element with the scene tag
//! (`docs/plans/css-for-3d.md`, C1): the key the compiler emitted for a
//! tag carrying `class` or `class:`, or the component's own, so anchors
//! share computed styles as template elements do; each mesh takes its
//! anchor's computed colour (C3).

mod support;

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
    // Given: the two wheels, one template site in a {for}, with no
    // class: both take the component's own key
    let rig = KeyedRig::new();

    // When
    let counts = rig.update();

    // Then: the root and the body are the only unkeyed elements
    assert_eq!(counts.styles_shared, 1, "{counts:?}");
    assert_eq!(counts.styles_unkeyed, 2, "{counts:?}");
}

#[test]
fn an_unclassed_mesh_takes_the_body_colour_only_by_inheritance() {
    // Given: as the racer HUD's `:global(body) { color }`, the page's
    // body is blue; `.body` declares red and `.braking` green, and the
    // wheels carry no class and declare nothing
    let rig = KeyedRig::new();
    rig.braking.set(true);

    // When
    rig.update();

    // Then: the braking body green, the wheels the body's blue
    let scene = rig.scene.borrow();
    let group = scene.children(scene.root()).unwrap()[0];
    let colours: Vec<Colour> = (scene.children(group).unwrap().iter())
        .map(|mesh| match scene.kind(*mesh).unwrap() {
            NodeKind::Mesh { colour, .. } => *colour,
            other => panic!("a mesh, found {other:?}"),
        })
        .collect();
    let [green, blue] =
        [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].map(|[r, g, b]| Colour::srgb(r, g, b).unwrap());
    assert_eq!(colours, [green, blue, blue]);
}

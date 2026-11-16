//! The components mounted through a `.wisp` page's component tags:
//! one scene node each, under the parent the anchor names, moved by
//! their signals.

mod support;

use support::{Rig, at};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{AxElement, Document, Viewport};
use vapors_scene3d_components::{Group, GroupProps};

#[test]
fn the_page_mounts_one_node_per_component_under_its_parent() {
    // When
    let rig = Rig::new();

    // Then: the root holds the camera and the group; the group the mesh
    let scene = rig.scene.borrow();
    let top = scene.children(scene.root()).unwrap().to_vec();
    assert_eq!(top.len(), 2, "camera and group");
    assert_eq!(scene.children(top[1]).unwrap().len(), 1, "the mesh");
    assert_eq!(scene.len(), 4);
}

#[test]
fn a_mesh_in_an_if_mounts_under_its_group_when_the_branch_is_built() {
    // Given
    let rig = Rig::new();

    // When
    rig.signals.shown.set(true);

    // Then
    let scene = rig.scene.borrow();
    let group = scene.children(scene.root()).unwrap()[1];
    assert_eq!(scene.children(group).unwrap().len(), 2);
}

#[test]
fn a_transform_change_recomputes_that_node_alone() {
    // Given
    let rig = Rig::new();
    rig.scene.borrow_mut().update();

    // When
    rig.signals.left.set(at(-1.0));

    // Then
    let mut scene = rig.scene.borrow_mut();
    let group = scene.children(scene.root()).unwrap()[1];
    let mesh = scene.children(group).unwrap()[0];
    assert_eq!(scene.update(), [mesh]);
}

#[test]
fn each_anchor_has_no_box_and_no_place_in_the_accessibility_tree() {
    // Given
    let rig = Rig::new();
    let viewport = Viewport {
        width: 64.0,
        height: 64.0,
        scale: 1.0,
    };
    rig.doc.borrow_mut().update(viewport).unwrap();

    // When
    let doc = rig.doc.borrow();
    let anchors = anchors(&doc, doc.body());

    // Then: scene, camera, group and mesh
    assert_eq!(anchors.len(), 4);
    let painted = doc.painted().unwrap();
    let boxed: Vec<_> = anchors
        .iter()
        .filter(|a| painted.border_box(**a).is_some())
        .collect();
    assert_eq!(boxed, [&anchors[0]; 0], "anchors with a box");
    let tree = doc.ax_tree().unwrap();
    let exposed: Vec<_> = flatten(&tree)
        .filter(|node| anchors.contains(&node.node))
        .collect();
    assert!(exposed.is_empty(), "anchors in {tree:#?}");
}

/// The elements under `node` holding a scene node.
fn anchors(doc: &Document, node: NodeId) -> Vec<NodeId> {
    let held = doc.current_attribute(node, "data-scene3d-node").is_some();
    let below = doc
        .children(node)
        .into_iter()
        .flat_map(|child| anchors(doc, child));
    held.then_some(node).into_iter().chain(below).collect()
}

fn flatten(tree: &[AxElement]) -> Box<dyn Iterator<Item = &AxElement> + '_> {
    Box::new(
        tree.iter()
            .flat_map(|node| std::iter::once(node).chain(flatten(&node.children))),
    )
}

#[test]
fn a_mesh_hidden_by_its_if_leaves_the_scene_and_comes_back_once() {
    // Given
    let rig = Rig::new();
    rig.signals.shown.set(true);

    // When
    rig.signals.shown.set(false);
    let hidden = rig.scene.borrow().len();
    rig.signals.shown.set(true);

    // Then: root, camera, group, mesh, and one if-mesh
    let scene = rig.scene.borrow();
    let group = scene.children(scene.root()).unwrap()[1];
    assert_eq!(hidden, 4);
    assert_eq!(scene.children(group).unwrap().len(), 2);
    assert_eq!(scene.len(), 5);
}

#[test]
fn unmounting_a_group_removes_its_node_and_its_anchor() {
    // Given: a group mounted from Rust in the scene's anchor
    let rig = Rig::new();
    let anchor = rig.page.roots()[1];
    let group = Group::mount(&rig.doc, anchor, GroupProps::default());
    let own = group.roots()[0];

    // When
    group.unmount(&rig.doc);

    // Then
    assert_eq!(rig.scene.borrow().len(), 4);
    assert!(!rig.doc.borrow().contains(own), "the anchor is gone");
}

#[test]
fn unmounting_the_page_removes_every_node_but_the_root() {
    // Given: the group's removal takes its mesh with it, so the mesh's
    // own removal finds its node stale
    let rig = Rig::new();

    // When
    rig.page.unmount(&rig.doc);

    // Then
    assert_eq!(rig.scene.borrow().len(), 1);
}

#[test]
fn a_page_dropped_after_its_app_removes_its_nodes_without_a_refusal() {
    // Given
    let rig = Rig::new();
    let (page, app, scene) = (rig.page, rig.app, rig.scene);

    // When: the app, and with it the event loop, goes first
    drop(app);
    drop(page);

    // Then: the page's nodes are gone, the root alone left
    assert_eq!(scene.borrow().len(), 1);
}

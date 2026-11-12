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

    // Then: the root holds the outer group; the outer group the inner
    let scene = rig.scene.borrow();
    let top = scene.children(scene.root()).unwrap().to_vec();
    assert_eq!(top.len(), 1, "the outer group");
    assert_eq!(scene.children(top[0]).unwrap().len(), 1, "the inner group");
    assert_eq!(scene.len(), 3);
}

#[test]
fn a_group_in_an_if_mounts_under_its_group_when_the_branch_is_built() {
    // Given
    let rig = Rig::new();

    // When
    rig.signals.shown.set(true);

    // Then
    let scene = rig.scene.borrow();
    let outer = scene.children(scene.root()).unwrap()[0];
    assert_eq!(scene.children(outer).unwrap().len(), 2);
}

#[test]
fn a_transform_change_recomputes_that_node_alone() {
    // Given
    let rig = Rig::new();
    rig.scene.borrow_mut().update();

    // When
    rig.signals.inner.set(at(-1.0));

    // Then
    let mut scene = rig.scene.borrow_mut();
    let outer = scene.children(scene.root()).unwrap()[0];
    let inner = scene.children(outer).unwrap()[0];
    assert_eq!(scene.update(), [inner]);
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

    // Then: the scene and two groups
    assert_eq!(anchors.len(), 3);
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
fn a_group_hidden_by_its_if_leaves_the_scene_and_comes_back_once() {
    // Given
    let rig = Rig::new();
    rig.signals.shown.set(true);

    // When
    rig.signals.shown.set(false);
    let hidden = rig.scene.borrow().len();
    rig.signals.shown.set(true);

    // Then: root, outer, inner, and one if-group
    let scene = rig.scene.borrow();
    let outer = scene.children(scene.root()).unwrap()[0];
    assert_eq!(hidden, 3);
    assert_eq!(scene.children(outer).unwrap().len(), 2);
    assert_eq!(scene.len(), 4);
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
    assert_eq!(rig.scene.borrow().len(), 3);
    assert!(!rig.doc.borrow().contains(own), "the anchor is gone");
}

#[test]
fn unmounting_the_page_removes_every_node_but_the_root() {
    // Given: the outer group's removal takes the inner group with it, so
    // the inner group's own removal finds its node stale
    let rig = Rig::new();

    // When
    rig.page.unmount(&rig.doc);

    // Then
    assert_eq!(rig.scene.borrow().len(), 1);
}

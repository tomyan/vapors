//! The components mounted through a `.wisp` page's component tags:
//! one scene node each, under the parent the anchor names, moved by
//! their signals.

mod support;

use support::{Rig, at};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{AxElement, Document, Viewport};

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

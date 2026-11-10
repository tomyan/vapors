use super::*;
use crate::{NodeKind, Quat, Vec3};
use glam::Vec4;
use std::f32::consts::FRAC_PI_2;

fn at(x: f32) -> Transform {
    Transform::from_translation(Vec3::new(x, 0.0, 0.0)).unwrap()
}

/// root > a > (a1, a2 > a2x), root > b
struct Tree {
    scene: Scene,
    a: NodeId,
    a2: NodeId,
    a2x: NodeId,
    b: NodeId,
}

fn tree() -> Tree {
    let mut scene = Scene::new();
    let root = scene.root();
    let a = scene.add(root, NodeKind::Group, at(1.0)).unwrap();
    scene.add(a, NodeKind::Mesh, at(1.0)).unwrap();
    let a2 = scene.add(a, NodeKind::Group, at(1.0)).unwrap();
    let a2x = scene.add(a2, NodeKind::Mesh, at(1.0)).unwrap();
    let b = scene.add(root, NodeKind::Mesh, at(1.0)).unwrap();
    scene.update();
    Tree {
        scene,
        a,
        a2,
        a2x,
        b,
    }
}

#[test]
fn a_child_world_matrix_is_parent_times_local() {
    // Given: a parent translated (1, 2, 3), turned a quarter about +Z and
    // scaled (2, 1, 1); a child 1 along its parent's +X
    let mut scene = Scene::new();
    let rotation = Quat::from_axis_angle(Vec3::Z, FRAC_PI_2);
    let local = Transform::new(Vec3::new(1.0, 2.0, 3.0), rotation, Vec3::new(2.0, 1.0, 1.0));
    let parent = scene
        .add(scene.root(), NodeKind::Group, local.unwrap())
        .unwrap();
    let child = scene.add(parent, NodeKind::Mesh, at(1.0)).unwrap();

    // When
    scene.update();
    let world = scene.world(child).unwrap();

    // Then: +X scales to 2 and turns to +Y, +Y turns to -X; the child's
    // origin lands at (1, 2, 3) + (0, 2, 0). In the other order
    // (local * parent) it would land at (2, 2, 3) with +X of length 2.
    let expected = Mat4::from_cols(
        Vec4::new(0.0, 2.0, 0.0, 0.0),
        Vec4::new(-1.0, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(1.0, 4.0, 3.0, 1.0),
    );
    assert!(world.abs_diff_eq(expected, 1e-6), "{world}");
}

#[test]
fn the_first_update_computes_every_node_then_none() {
    // Given
    let mut scene = tree().scene;

    // When
    let again = scene.update();

    // Then: the tree's update computed all six; nothing has changed since
    assert_eq!(scene.len(), 6);
    assert_eq!(again, Vec::<NodeId>::new());
}

#[test]
fn a_leaf_change_recomputes_that_leaf_alone() {
    // Given
    let mut tree = tree();

    // When
    tree.scene.set_transform(tree.b, at(5.0)).unwrap();
    let changed = tree.scene.update();

    // Then
    assert_eq!(changed, vec![tree.b]);
    let world = tree.scene.world(tree.b).unwrap();
    assert_eq!(world.w_axis.x, 5.0);
}

#[test]
fn a_parent_change_recomputes_its_subtree_only() {
    // Given: a child of a2 also changed, so it is marked twice over
    let mut tree = tree();
    tree.scene.set_transform(tree.a2x, at(2.0)).unwrap();
    tree.scene.set_transform(tree.a2, at(3.0)).unwrap();

    // When
    let changed = tree.scene.update();

    // Then: a2 and a2x, each once; not a, its other child, nor b
    assert_eq!(changed, vec![tree.a2, tree.a2x]);
    let world = tree.scene.world(tree.a2x).unwrap();
    assert_eq!(world.w_axis.x, 1.0 + 3.0 + 2.0);
}

#[test]
fn a_world_matrix_is_refused_until_update_runs() {
    // Given: an ancestor of a2x changed
    let mut tree = tree();
    tree.scene.set_transform(tree.a, at(9.0)).unwrap();

    // When
    let result = tree.scene.world(tree.a2x);

    // Then
    assert_eq!(result, Err(StoreError::WorldPending(tree.a2x)));
}

#[test]
fn a_moved_node_composes_with_its_new_parent() {
    // Given
    let mut tree = tree();

    // When: a2 (world x 2) moves under b (world x 1)
    tree.scene.reparent(tree.a2, tree.b).unwrap();
    let changed = tree.scene.update();

    // Then: only the moved subtree is recomputed
    assert_eq!(changed, vec![tree.a2, tree.a2x]);
    assert_eq!(tree.scene.parent(tree.a2), Ok(Some(tree.b)));
    assert_eq!(tree.scene.children(tree.b), Ok(&[tree.a2][..]));
    assert_eq!(tree.scene.children(tree.a).unwrap().len(), 1);
    assert_eq!(tree.scene.world(tree.a2x).unwrap().w_axis.x, 3.0);
}

#[test]
fn a_move_under_its_own_subtree_is_refused() {
    // Given
    let mut tree = tree();

    // When
    let result = tree.scene.reparent(tree.a, tree.a2x);

    // Then: refused, and nothing moved
    let cycle = StoreError::Cycle {
        node: tree.a,
        parent: tree.a2x,
    };
    assert_eq!(result, Err(cycle));
    assert_eq!(tree.scene.parent(tree.a), Ok(Some(tree.scene.root())));
    assert_eq!(
        tree.scene.reparent(tree.a, tree.a),
        Err(StoreError::Cycle {
            node: tree.a,
            parent: tree.a
        })
    );
}

#[test]
fn a_node_changed_then_removed_is_not_recomputed() {
    // Given
    let mut tree = tree();
    tree.scene.set_transform(tree.a2x, at(4.0)).unwrap();
    tree.scene.remove(tree.a2).unwrap();

    // When
    let changed = tree.scene.update();

    // Then
    assert_eq!(changed, Vec::<NodeId>::new());
}

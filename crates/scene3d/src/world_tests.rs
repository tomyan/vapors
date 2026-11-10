use super::*;
use crate::test_support::mesh;
use crate::{NodeKind, Quat, Vec3};
use glam::Vec4;
use std::f32::consts::{FRAC_1_SQRT_2, FRAC_PI_2, FRAC_PI_4};

pub(super) fn at(x: f32) -> Transform {
    Transform::from_translation(Vec3::new(x, 0.0, 0.0)).unwrap()
}

/// root > a > (a1, a2 > a2x), root > b
pub(super) struct Tree {
    pub(super) scene: Scene,
    /// What the first update returned.
    pub(super) first: Vec<NodeId>,
    pub(super) a: NodeId,
    pub(super) a1: NodeId,
    pub(super) a2: NodeId,
    pub(super) a2x: NodeId,
    pub(super) b: NodeId,
}

pub(super) fn tree() -> Tree {
    let mut scene = Scene::new();
    let root = scene.root();
    let a = scene.add(root, NodeKind::Group, at(1.0)).unwrap();
    let a1 = scene.add(a, mesh(), at(1.0)).unwrap();
    let a2 = scene.add(a, NodeKind::Group, at(1.0)).unwrap();
    let a2x = scene.add(a2, mesh(), at(1.0)).unwrap();
    let b = scene.add(root, mesh(), at(1.0)).unwrap();
    let first = scene.update();
    Tree {
        scene,
        first,
        a,
        a1,
        a2,
        a2x,
        b,
    }
}

#[test]
fn a_child_world_matrix_is_parent_times_local() {
    // Given: a parent translated (1, 2, 3), turned a quarter about +Z and
    // scaled (2, 1, 1); a child 1 along its parent's +X, turned an eighth
    // about +Z
    let mut scene = Scene::new();
    let rotation = Quat::from_axis_angle(Vec3::Z, FRAC_PI_2);
    let local = Transform::new(Vec3::new(1.0, 2.0, 3.0), rotation, Vec3::new(2.0, 1.0, 1.0));
    let parent = scene
        .add(scene.root(), NodeKind::Group, local.unwrap())
        .unwrap();
    let eighth = Quat::from_axis_angle(Vec3::Z, FRAC_PI_4);
    let local = Transform::new(Vec3::X, eighth, Vec3::ONE).unwrap();
    let child = scene.add(parent, mesh(), local).unwrap();

    // When
    scene.update();
    let world = scene.world(child).unwrap();

    // Then: with s = sqrt(1/2), the parent's columns (scale, then turn)
    // are (0, 2, 0), (-1, 0, 0), (0, 0, 1), origin (1, 2, 3); the child's
    // are (s, s, 0), (-s, s, 0), (0, 0, 1), origin (1, 0, 0). The product
    // parent * local has columns
    //   x: s*(0, 2, 0) + s*(-1, 0, 0) = (-s, 2s, 0)
    //   y: -s*(0, 2, 0) + s*(-1, 0, 0) = (-s, -2s, 0)
    //   origin: (0, 2, 0) + (1, 2, 3) = (1, 4, 3)
    // The columns are not orthogonal (the parent's scale shears the
    // child's turn), and local * parent gives different ones.
    let s = FRAC_1_SQRT_2;
    let expected = Mat4::from_cols(
        Vec4::new(-s, 2.0 * s, 0.0, 0.0),
        Vec4::new(-s, -2.0 * s, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(1.0, 4.0, 3.0, 1.0),
    );
    assert!(world.abs_diff_eq(expected, 1e-6), "{world}");
}

#[test]
fn the_first_update_computes_every_node_then_none() {
    // Given
    let mut tree = tree();

    // When
    let again = tree.scene.update();

    // Then: the first update computed all six, parents before children;
    // nothing has changed since
    let root = tree.scene.root();
    let expected = vec![root, tree.a, tree.b, tree.a1, tree.a2, tree.a2x];
    assert_eq!(tree.first, expected);
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

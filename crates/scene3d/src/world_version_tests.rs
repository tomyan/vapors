use super::tests::{at, tree};
use crate::StoreError;

#[test]
fn an_update_advances_the_world_version_of_each_node_it_recomputes_only() {
    // Given: a2 moved, so a2 and a2x are recomputed and b is not
    let mut tree = tree();
    let before = [tree.a2, tree.a2x, tree.b].map(|id| tree.scene.world_version(id).unwrap());
    tree.scene.set_transform(tree.a2, at(3.0)).unwrap();

    // When
    tree.scene.update();

    // Then
    let after = [tree.a2, tree.a2x, tree.b].map(|id| tree.scene.world_version(id).unwrap());
    assert!(after[0] > before[0], "a2: {before:?} then {after:?}");
    assert!(after[1] > before[1], "a2x: {before:?} then {after:?}");
    assert_eq!(after[2], before[2], "b: {before:?} then {after:?}");
}

#[test]
fn an_update_with_nothing_changed_keeps_every_world_version() {
    // Given
    let mut tree = tree();
    let before = tree.scene.world_version(tree.a2x).unwrap();

    // When
    tree.scene.update();

    // Then
    assert_eq!(tree.scene.world_version(tree.a2x), Ok(before));
}

#[test]
fn a_world_version_is_refused_until_update_runs() {
    // Given: an ancestor of a2x changed
    let mut tree = tree();
    tree.scene.set_transform(tree.a, at(9.0)).unwrap();

    // When
    let result = tree.scene.world_version(tree.a2x);

    // Then
    assert_eq!(result, Err(StoreError::WorldPending(tree.a2x)));
}

#[test]
fn a_removed_node_has_no_world_version() {
    // Given
    let mut tree = tree();
    tree.scene.remove(tree.b).unwrap();

    // When
    let result = tree.scene.world_version(tree.b);

    // Then
    assert_eq!(result, Err(StoreError::Stale(tree.b)));
}

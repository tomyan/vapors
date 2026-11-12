use super::tests::{at, tree};
use crate::{NodeKind, StoreError};

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
fn a_move_under_a_removed_parent_is_refused_and_nothing_moves() {
    // Given
    let mut tree = tree();
    let root = tree.scene.root();
    let gone = tree.scene.add(root, NodeKind::Group, at(1.0)).unwrap();
    tree.scene.remove(gone).unwrap();

    // When
    let result = tree.scene.reparent(tree.a2, gone);

    // Then
    assert_eq!(result, Err(StoreError::Stale(gone)));
    assert_eq!(tree.scene.parent(tree.a2), Ok(Some(tree.a)));
    assert_eq!(tree.scene.children(tree.a).unwrap().len(), 2);
}

#[test]
fn a_node_moves_under_the_root() {
    // Given
    let mut tree = tree();
    let root = tree.scene.root();

    // When
    tree.scene.reparent(tree.a2, root).unwrap();
    let changed = tree.scene.update();

    // Then: a2 goes last among the root's children and composes with it
    assert_eq!(
        tree.scene.children(root),
        Ok(&[tree.a, tree.b, tree.a2][..])
    );
    assert_eq!(changed, vec![tree.a2, tree.a2x]);
    assert_eq!(tree.scene.world(tree.a2x).unwrap().w_axis.x, 2.0);
}

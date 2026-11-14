//! Meshes in a keyed `{for}` (`docs/plans/3d-2.md` row 5): a row keeps
//! its scene node across a reorder; a row dropped removes its node, and
//! a row added adds one, the others untouched.

mod support;

use std::collections::BTreeMap;

use support::rows::RowsRig;
use vapors_scene3d::NodeId;

/// Each mesh node under the group, by the row number its x is.
fn nodes_by_row(rig: &RowsRig) -> BTreeMap<i32, NodeId> {
    let scene = rig.scene.borrow();
    let group = scene.children(scene.root()).unwrap()[0];
    (scene.children(group).unwrap().iter())
        .map(|&node| {
            let x = scene.transform(node).unwrap().matrix().w_axis.x;
            #[allow(clippy::cast_possible_truncation)] // a row number
            (x as i32, node)
        })
        .collect()
}

#[test]
fn a_reorder_keeps_each_rows_scene_node() {
    // Given
    let rig = RowsRig::new(vec![1, 2, 3]);
    let before = nodes_by_row(&rig);

    // When
    rig.rows.set(vec![3, 1, 2]);

    // Then
    assert_eq!(nodes_by_row(&rig), before);
    assert_eq!(
        rig.scene.borrow().len(),
        2 + 3,
        "root, group and three meshes"
    );
}

#[test]
fn a_dropped_row_removes_its_node_alone_and_an_added_row_adds_one() {
    // Given
    let rig = RowsRig::new(vec![1, 2, 3]);
    let before = nodes_by_row(&rig);

    // When
    rig.rows.set(vec![3, 4, 1]);

    // Then
    let after = nodes_by_row(&rig);
    assert_eq!((after[&1], after[&3]), (before[&1], before[&3]));
    assert!(!after.contains_key(&2) && after.contains_key(&4));
    let scene = rig.scene.borrow();
    assert!(scene.kind(before[&2]).is_err(), "row 2's node is removed");
}

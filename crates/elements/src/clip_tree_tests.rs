use super::ClipTree;

#[test]
#[should_panic(expected = "slot 1 has clip parent 0 but is not among its dependents")]
fn freeing_a_slot_missing_from_its_parents_group_is_refused() {
    // Given: slot 1's parent is 0, whose group does not hold it
    let mut tree = ClipTree {
        parent: vec![None, Some(0)],
        depth: vec![0, 1],
        dependents: vec![Vec::new(), Vec::new()],
    };

    // When
    tree.spliced(2, &[], std::iter::once(1));
}

#[test]
fn freeing_a_slot_and_its_parent_together_leaves_both_groups_empty() {
    // Given: 0 above 1 above 2
    let mut tree = ClipTree {
        parent: vec![None, Some(0), Some(1)],
        depth: vec![0, 1, 2],
        dependents: vec![vec![1], vec![2], Vec::new()],
    };

    // When: 1 freed before its child 2
    tree.spliced(3, &[], [1, 2].into_iter());

    // Then
    assert!(tree.dependents.iter().all(Vec::is_empty), "{tree:?}");
    assert_eq!(tree.parent, [None, None, None]);
}

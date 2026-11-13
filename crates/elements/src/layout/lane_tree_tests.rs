//! The lane tree on flex summaries: children put in, taken out and
//! resized, where each starts, and which an interval meets.

use super::{Handle, LaneTree};
use crate::layout::lane_bounds::Bounds;
use crate::layout::lane_flex_summary::{FlexItem, FlexSummary};

/// An item `size` tall with no margins, reaching its own bottom.
fn item(size: f64) -> FlexSummary {
    FlexSummary::leaf(FlexItem {
        margins: [0.0, 0.0],
        size,
        basis: size,
        content: size,
        bounds: Bounds::span(0.0, size),
    })
}

/// `count` items 10 tall, appended in order, with a gap of 2.
fn appended(count: usize) -> (LaneTree<FlexSummary>, Vec<Handle>) {
    let mut tree = LaneTree::new(2.0);
    let mut handles: Vec<Handle> = Vec::new();
    for _ in 0..count {
        let handle = tree.insert_after(handles.last().copied(), item(10.0));
        handles.push(handle);
    }
    (tree, handles)
}

#[test]
fn each_child_starts_after_those_before_it_and_their_gaps() {
    // Given / When: more children than one node holds, so it splits
    let (tree, handles) = appended(100);

    // Then
    for (index, &handle) in handles.iter().enumerate() {
        let index_f64 = f64::from(u32::try_from(index).expect("100 children"));
        assert_eq!(tree.start(handle), 12.0 * index_f64, "child {index}");
    }
    assert_eq!(tree.total().outer(2.0), 100.0 * 12.0 - 2.0);
    assert_eq!(tree.handles(), handles);
}

#[test]
fn an_insert_in_the_middle_moves_only_the_later_starts() {
    // Given
    let (mut tree, handles) = appended(50);

    // When
    let added = tree.insert_after(Some(handles[24]), item(30.0));

    // Then
    assert_eq!(tree.start(added), 25.0 * 12.0);
    assert_eq!(tree.start(handles[24]), 24.0 * 12.0);
    assert_eq!(tree.start(handles[25]), 25.0 * 12.0 + 32.0);
}

#[test]
fn an_insert_first_starts_at_zero() {
    // Given
    let (mut tree, handles) = appended(20);

    // When
    let first = tree.insert_after(None, item(5.0));

    // Then
    assert_eq!(tree.start(first), 0.0);
    assert_eq!(tree.start(handles[0]), 7.0);
    assert_eq!(tree.handles()[0], first);
}

#[test]
fn a_remove_closes_the_gap_and_frees_emptied_nodes() {
    // Given
    let (mut tree, handles) = appended(40);

    // When: every child but the last two taken out
    for &handle in &handles[..38] {
        tree.remove(handle);
    }

    // Then
    assert_eq!(tree.start(handles[38]), 0.0);
    assert_eq!(tree.start(handles[39]), 12.0);
    assert!(
        tree.nodes[tree.root].leaf,
        "a root with one child is dropped"
    );
}

#[test]
fn a_resized_child_moves_the_later_starts() {
    // Given
    let (mut tree, handles) = appended(30);

    // When
    tree.set(handles[3], item(40.0));

    // Then
    assert_eq!(tree.start(handles[4]), 4.0 * 12.0 + 30.0);
    assert_eq!(tree.summary(handles[3]), item(40.0));
}

#[test]
fn between_gives_the_children_an_interval_meets_in_order() {
    // Given: children at 0, 12, 24, ...
    let (tree, handles) = appended(200);

    // When
    let shown = tree.between(100.0, 130.0);

    // Then: 96..106, 108..118, 120..130
    assert_eq!(shown, handles[8..11]);
}

#[test]
fn starts_from_gives_each_later_child_with_its_start() {
    // Given: more children than one leaf holds
    let (tree, handles) = appended(60);

    // When
    let from = tree.starts_from(handles[37]);

    // Then: handle 37 on, each at the start `start` gives
    let expected: Vec<(Handle, f64)> = handles[37..]
        .iter()
        .map(|&handle| (handle, tree.start(handle)))
        .collect();
    assert_eq!(from, expected);
    assert_eq!(tree.starts_from(handles[59]).len(), 1);
}

#[test]
#[should_panic(expected = "is not in the tree")]
fn a_removed_handle_is_refused() {
    let (mut tree, handles) = appended(3);
    tree.remove(handles[1]);
    tree.start(handles[1]);
}

#[test]
#[should_panic(expected = "is not in the tree")]
fn a_removed_handle_is_refused_once_its_index_names_a_new_child() {
    // Given: a remove, then an insert that takes the freed index
    let (mut tree, handles) = appended(3);
    tree.remove(handles[1]);
    let added = tree.insert_after(Some(handles[0]), item(10.0));
    assert_ne!(added, handles[1], "the new child's handle differs");

    // When / Then: the old handle does not name the new child
    tree.start(handles[1]);
}

#[test]
fn a_freed_index_is_reused_on_a_new_generation() {
    // Given
    let (mut tree, handles) = appended(3);
    tree.remove(handles[1]);

    // When
    let added = tree.insert_after(Some(handles[0]), item(30.0));

    // Then: the index is reused, the handle is new, and it works
    assert_eq!(tree.leaf_of.len(), 3);
    assert_ne!(added, handles[1]);
    assert_eq!(tree.start(added), 12.0);
    assert_eq!(tree.handles(), vec![handles[0], added, handles[2]]);
}

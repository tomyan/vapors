//! A new root: the slots start afresh, whatever the old tree held.

use super::*;

/// `placed` holds `nodes` in slots 0 up in order, and no vacant slot.
fn assert_fresh(placed: &Placed, nodes: &[u64]) {
    assert_eq!(placed.boxes.len(), nodes.len());
    for (slot, &node) in nodes.iter().enumerate() {
        assert_eq!(slot_of(placed, node), Some(slot), "node {node}");
        assert!(
            placed.boxes[slot].content == Content::Element(NodeId(node)),
            "slot {slot}"
        );
    }
    assert_eq!(placed.slots.free, Vec::<usize>::new());
}

#[test]
fn a_new_root_starts_from_slot_zero_with_no_vacant_slots() {
    // Given
    let first = place_each(vec![tree(&[(1, &[1, 2]), (2, &[]), (3, &[])])]);

    // When
    let second = first.slots.place(tree(&[(9, &[1]), (2, &[])]));

    // Then
    assert_fresh(&second, &[9, 2]);
    assert_eq!(second.boxes[0].children, vec![1]);
}

#[test]
fn a_descendant_that_becomes_the_root_moves_to_slot_zero() {
    // Given: node 3 is in slot 2
    let first = place_each(vec![tree(&[(1, &[1, 2]), (2, &[]), (3, &[3]), (4, &[])])]);

    // When
    let second = first.slots.place(tree(&[(3, &[1]), (4, &[])]));

    // Then
    assert_fresh(&second, &[3, 4]);
}

#[test]
fn an_old_root_that_becomes_a_child_leaves_slot_zero() {
    // Given
    let first = place_each(vec![tree(&[(1, &[1]), (2, &[])])]);

    // When
    let second = first.slots.place(tree(&[(5, &[1]), (1, &[2]), (2, &[])]));

    // Then
    assert_fresh(&second, &[5, 1, 2]);
    assert_eq!(second.boxes[1].children, vec![2]);
}

#[test]
#[should_panic(expected = "slot 7 is vacant")]
fn asking_a_vacant_slot_for_its_element_is_refused() {
    // Given
    let vacant = LayoutBox::vacant();

    // When
    let _ = vacant.element_in(7);
}

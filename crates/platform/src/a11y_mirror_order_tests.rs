//! The order model's reconcile moves only what is out of place.

use super::{Move, Order};

/// Four nodes under the root, in order.
fn four() -> Order {
    let mut order = Order::default();
    for node in 1..=4 {
        order.place(node, None, None);
    }
    order
}

#[test]
fn the_same_order_moves_nothing() {
    // Given
    let mut order = four();

    // When
    let moves = order.reconcile(&[(1, None), (2, None), (3, None), (4, None)]);

    // Then
    assert_eq!(moves, []);
}

#[test]
fn a_node_moved_to_the_front_is_the_one_move() {
    // Given
    let mut order = four();

    // When: 4 first
    let moves = order.reconcile(&[(4, None), (1, None), (2, None), (3, None)]);

    // Then
    assert_eq!(
        moves,
        [Move {
            node: 4,
            parent: None,
            before: Some(1)
        }]
    );
    assert_eq!(order.ancestors(4).count(), 0);
}

#[test]
fn a_new_node_and_one_moved_under_another_parent_are_placed_and_the_rest_left() {
    // Given
    let mut order = four();

    // When: 5 is new between 1 and 2; 3 goes under 2; 4 stays last
    let moves = order.reconcile(&[(1, None), (5, None), (2, None), (3, Some(2)), (4, None)]);

    // Then: 5 before 2, 3 appended under 2; 4 is left alone, since 3
    // leaves the root
    let root: Vec<_> = moves.iter().filter(|m| m.parent.is_none()).collect();
    assert_eq!(root.len(), 1, "{moves:?}");
    assert_eq!((root[0].node, root[0].before), (5, Some(2)));
    assert!(moves.contains(&Move {
        node: 3,
        parent: Some(2),
        before: None
    }));
    assert_eq!(moves.len(), 2, "{moves:?}");
    assert_eq!(order.ancestors(3).collect::<Vec<_>>(), [2]);
}

#[test]
fn a_removal_takes_the_subtree_out_of_the_model() {
    // Given: 3 under 2
    let mut order = four();
    order.place(3, Some(2), None);

    // When
    order.remove(2);

    // Then: 3 has no parent any more, and the root holds 1 and 4
    assert_eq!(order.ancestors(3).count(), 0);
    assert_eq!(order.reconcile(&[(1, None), (4, None)]), []);
}

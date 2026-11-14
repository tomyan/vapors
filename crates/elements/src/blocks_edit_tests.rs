use super::tests::{counted, items};
use super::{Blocks, CAP, Pos};

fn items_i64(blocks: &Blocks<i64>) -> Vec<i64> {
    blocks.iter().copied().collect()
}

#[test]
fn insert_puts_the_item_at_the_position_and_returns_where_it_is() {
    // Given
    let mut blocks = Blocks::from_sorted(vec![0, 2, 4]);

    // When
    let placed = blocks.insert(blocks.partition_point(|&item| item < 3), 3);
    let placed_at = blocks.get(placed).copied();
    let first = blocks.insert(blocks.partition_point(|&item| item < 0), -1);
    let first_at = blocks.get(first).copied();
    let last = blocks.insert(blocks.end(), 9);

    // Then
    assert_eq!(items_i64(&blocks), vec![-1, 0, 2, 3, 4, 9]);
    assert_eq!((placed_at, first_at), (Some(3), Some(-1)));
    assert_eq!(blocks.get(last), Some(&9));
    assert_eq!(blocks.block_count(), 1, "appends fill the last block");
}

#[test]
fn insert_into_a_full_block_splits_it_and_keeps_the_order() {
    // Given
    let mut blocks = counted(CAP);
    assert_eq!(blocks.block_count(), 1);

    // When
    let at = blocks.partition_point(|&item| item < CAP / 2);
    let placed = blocks.insert(at, usize::MAX);

    // Then
    assert_eq!(blocks.block_count(), 2);
    assert_eq!(blocks.get(placed), Some(&usize::MAX));
    let mut expected: Vec<usize> = (0..CAP).collect();
    expected.insert(CAP / 2, usize::MAX);
    assert_eq!(items(&blocks), expected);
}

#[test]
fn remove_returns_the_item_and_the_position_of_the_next() {
    // Given
    let mut blocks = counted(2 * CAP);

    // When
    let (last_of_first, next) = blocks.remove(blocks.partition_point(|&item| item < CAP - 1));

    // Then
    assert_eq!(last_of_first, CAP - 1);
    assert_eq!(blocks.get(next), Some(&CAP));
    assert_eq!(blocks.len(), 2 * CAP - 1);
}

#[test]
fn removing_the_only_item_of_a_block_drops_the_block() {
    // Given
    let mut blocks = counted(CAP + 1);
    assert_eq!(blocks.block_count(), 2);

    // When
    let (alone, next) = blocks.remove(blocks.partition_point(|&item| item < CAP));
    let (first, after_first) = blocks.remove(Pos::default());

    // Then
    assert_eq!((alone, first), (CAP, 0));
    assert_eq!(next, blocks.end());
    assert_eq!(blocks.get(after_first), Some(&1));
    assert_eq!(blocks.block_count(), 1);
    assert_eq!(blocks.len(), CAP - 1);
}

#[test]
fn removing_every_item_leaves_no_block() {
    // Given
    let mut blocks = Blocks::from_sorted(vec![1, 2]);

    // When
    let (_, next) = blocks.remove(Pos::default());
    let (_, at_end) = blocks.remove(next);

    // Then
    assert_eq!(at_end, blocks.end());
    assert!(blocks.is_empty());
    assert_eq!(blocks.block_count(), 0);
}

#[test]
fn an_insert_or_remove_moves_at_most_a_block_of_items_and_the_block_pointers() {
    // Given
    let mut blocks = counted(40 * CAP);

    // When
    let at = blocks.partition_point(|&item| item < 20 * CAP + 3);
    let placed = blocks.insert(at, 0);
    let after_insert = blocks.moved();
    blocks.remove(placed);
    let after_remove = blocks.moved() - after_insert;

    // Then
    let most = CAP + blocks.block_count();
    assert!(after_insert <= most, "insert moved {after_insert} items");
    assert!(after_remove <= most, "remove moved {after_remove} items");
    assert!(
        after_insert > 0 && after_remove > 0,
        "a middle insert moves the block's tail"
    );
}

#[test]
fn a_split_counts_the_block_pointers_moved_after_it() {
    // Given: three full blocks
    let mut blocks = counted(3 * CAP);

    // When: an item goes in at the front of the first, which splits
    blocks.insert(Pos::default(), usize::MAX);

    // Then: the upper half, the two block pointers after the first
    // block, and the lower half's tail after the item
    assert_eq!(blocks.block_count(), 4);
    assert_eq!(blocks.moved(), CAP / 2 + 2 + CAP / 2);
}

#[test]
fn an_emptied_block_counts_the_block_pointers_moved_after_it() {
    // Given: three full blocks, the middle one down to one item
    let mut blocks = counted(3 * CAP);
    let first_of_middle = blocks.partition_point(|&item| item < CAP);
    for _ in 1..CAP {
        blocks.remove(first_of_middle);
    }
    assert_eq!(blocks.block_count(), 3);
    let before = blocks.moved();

    // When: its last item goes
    let (gone, next) = blocks.remove(first_of_middle);

    // Then: no tail, one block pointer (the last block's) moved
    assert_eq!(gone, 2 * CAP - 1);
    assert_eq!(blocks.get(next), Some(&(2 * CAP)));
    assert_eq!(blocks.moved() - before, 1);
}

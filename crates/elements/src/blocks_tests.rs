use super::{Blocks, CAP, Pos};

/// `0..len`, in order.
pub(super) fn counted(len: usize) -> Blocks<usize> {
    Blocks::from_sorted((0..len).collect())
}

pub(super) fn items(blocks: &Blocks<usize>) -> Vec<usize> {
    blocks.iter().copied().collect()
}

#[test]
fn from_sorted_keeps_the_order_across_blocks() {
    // Given
    let len = 3 * CAP + 5;

    // When
    let blocks = counted(len);

    // Then
    assert_eq!(blocks.len(), len);
    assert_eq!(items(&blocks), (0..len).collect::<Vec<_>>());
    assert!(blocks.block_count() >= 3, "{} blocks", blocks.block_count());
}

#[test]
fn from_sorted_holds_about_one_slot_per_item() {
    // Given: 10,000 rows' entries, as the extents index holds
    let len = 10_000;

    // When
    let blocks = counted(len);

    // Then: each block holds at most a block's slots, not the list's
    assert!(
        blocks.capacity() <= len + CAP,
        "{} slots for {len} items",
        blocks.capacity()
    );
}

#[test]
fn partition_point_finds_the_first_item_failing_the_predicate_in_any_block() {
    // Given
    let blocks = counted(3 * CAP);

    // When
    let in_second_block = blocks.partition_point(|&item| item < CAP + 7);
    let at_a_boundary = blocks.partition_point(|&item| item < CAP);
    let past_every = blocks.partition_point(|_| true);
    let before_every = blocks.partition_point(|_| false);

    // Then
    assert_eq!(blocks.get(in_second_block), Some(&(CAP + 7)));
    assert_eq!(blocks.get(at_a_boundary), Some(&CAP));
    assert_eq!(blocks.get(past_every), None);
    assert_eq!(past_every, blocks.end());
    assert_eq!(blocks.get(before_every), Some(&0));
}

#[test]
fn binary_search_names_the_position_found_or_the_insertion_point() {
    // Given
    let blocks = Blocks::from_sorted((0..2 * CAP).map(|item| item * 2).collect());

    // When
    let found = blocks.binary_search_by(|item| item.cmp(&(2 * CAP + 4)));
    let missing = blocks.binary_search_by(|item| item.cmp(&(2 * CAP + 5)));

    // Then
    assert_eq!(blocks.get(found.expect("present")), Some(&(2 * CAP + 4)));
    let at = missing.expect_err("absent");
    assert_eq!(blocks.get(at), Some(&(2 * CAP + 6)));
}

#[test]
fn prev_crosses_a_block_boundary_and_is_none_before_the_first() {
    // Given
    let blocks = counted(2 * CAP);

    // When
    let first_of_second = blocks.partition_point(|&item| item < CAP);
    let before = blocks.prev(first_of_second).expect("a block before");
    let before_first = blocks.prev(Pos::default());
    let last = blocks.prev(blocks.end()).expect("not empty");

    // Then
    assert_eq!(blocks.get(before), Some(&(CAP - 1)));
    assert_eq!(before_first, None);
    assert_eq!(blocks.get(last), Some(&(2 * CAP - 1)));
}

#[test]
fn iter_from_and_iter_mut_from_run_on_across_blocks() {
    // Given
    let mut blocks = counted(2 * CAP + 2);
    let from = blocks.partition_point(|&item| item < CAP - 1);

    // When
    for item in blocks.iter_mut_from(from) {
        *item += 1000;
    }
    let read: Vec<usize> = blocks.iter_from(from).copied().collect();

    // Then
    let expected: Vec<usize> = (CAP - 1..2 * CAP + 2).map(|item| item + 1000).collect();
    assert_eq!(read, expected);
    assert_eq!(blocks.get(Pos::default()), Some(&0));
}

#[test]
fn equality_reads_the_items_not_the_block_boundaries() {
    // Given
    let sorted = counted(2 * CAP);
    let mut grown = counted(2 * CAP);

    // When
    let third = CAP / 3;
    grown.insert(grown.partition_point(|&item| item < third), third);
    grown.remove(grown.partition_point(|&item| item < third));

    // Then
    assert_ne!(
        sorted.block_count(),
        grown.block_count(),
        "the point of the test"
    );
    assert_eq!(sorted, grown);
    assert_ne!(sorted, counted(2 * CAP - 1));
}

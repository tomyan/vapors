//! The slots a blocked list holds stay within a bound of its items on
//! every path: built, grown at the end, the front or the middle, and
//! emptied (the bound, `blocks.rs`: at most four slots an item and one
//! block more, for the last block a push starts).

use super::tests::counted;
use super::{Blocks, CAP};

const LEN: usize = 10_000;

/// Slots the blocks hold, used or not.
fn capacity(blocks: &Blocks<usize>) -> usize {
    blocks.chunks.iter().map(Vec::capacity).sum()
}

/// The bound: four slots an item, and one block.
fn bound(blocks: &Blocks<usize>) -> usize {
    4 * blocks.len() + CAP
}

fn grown_at(at: impl Fn(&Blocks<usize>) -> super::Pos) -> Blocks<usize> {
    let mut blocks = Blocks::default();
    for item in 0..LEN {
        let pos = at(&blocks);
        blocks.insert(pos, item);
    }
    blocks
}

#[test]
fn from_sorted_holds_about_one_slot_per_item() {
    // When: 10,000 rows' entries, as the extents index holds
    let blocks = counted(LEN);

    // Then: each block holds at most a block's slots, not the list's
    assert!(
        capacity(&blocks) <= LEN + CAP,
        "{} slots for {LEN} items",
        capacity(&blocks)
    );
}

#[test]
fn a_list_grown_at_the_end_the_front_or_the_middle_stays_in_bound() {
    let grown = [
        ("end", grown_at(Blocks::end)),
        ("front", grown_at(|_| super::Pos::default())),
        (
            "middle",
            grown_at(|blocks| blocks.partition_point(|&item| item < LEN / 2)),
        ),
    ];
    for (place, blocks) in grown {
        assert_eq!(blocks.len(), LEN);
        assert!(
            capacity(&blocks) <= bound(&blocks),
            "grown at the {place}: {} slots for {LEN} items",
            capacity(&blocks)
        );
    }
}

#[test]
fn a_list_with_most_items_removed_gives_its_slots_back() {
    // Given
    let mut blocks = counted(LEN);

    // When: 127 of every 128 removed
    for item in 0..LEN {
        if item % CAP != 0 {
            let pos = blocks
                .binary_search_by(|kept| kept.cmp(&item))
                .expect("held");
            blocks.remove(pos);
        }
    }

    // Then
    assert!(
        capacity(&blocks) <= bound(&blocks),
        "{} slots for {} items in {} blocks",
        capacity(&blocks),
        blocks.len(),
        blocks.block_count()
    );
}

//! Which of a lane's children are laid out again.

use rustc_hash::{FxHashMap, FxHashSet};

use super::again;
use crate::layout::lane_block_summary::BlockSummary;
use crate::layout::lane_tree::{Handle, LaneTree};

/// Handles for children in `slots`, from a tree that holds them.
fn handles(slots: &[usize]) -> FxHashMap<usize, Handle> {
    let mut tree = LaneTree::<BlockSummary>::new(0.0);
    let mut previous = None;
    slots
        .iter()
        .map(|&slot| {
            let handle = tree.insert_after(previous, BlockSummary::default());
            previous = Some(handle);
            (slot, handle)
        })
        .collect()
}

#[test]
fn a_stale_child_that_reads_the_heights_is_laid_out_again_once() {
    // Given: children 1 to 4; 2 and 9 stale (9 not a child); 2 and 3
    // read the height bases
    let stale: FxHashSet<usize> = [2, 9].into_iter().collect();
    let readers: FxHashSet<usize> = [2, 3].into_iter().collect();
    let children = handles(&[1, 2, 3, 4]);

    // When
    let changed = again(&stale, &children, Some(&readers));
    let unchanged = again(&stale, &children, None);

    // Then
    assert_eq!(changed, [2, 3]);
    assert_eq!(unchanged, [2]);
}

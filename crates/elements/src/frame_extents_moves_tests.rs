//! The extents index's patches move at most a block of entries
//! (`blocks.rs`), whatever the frame's size.

use super::Extents;
use super::tests::{extents, painted_again, part, row};
use crate::frames::PAGE;

/// Panics when `extents`' running block total is not the blocks its
/// frames' entries hold.
pub(super) fn assert_blocks_total(extents: &mut Extents) {
    let held: usize = extents
        .frames
        .values()
        .map(|index| index.entries.block_count())
        .sum();
    assert_eq!(
        extents.take_counts().blocks,
        held,
        "the running block total is not the blocks held (frame_extents_patch.rs `noted`)"
    );
}

#[test]
fn a_step_that_starts_drawing_among_many_moves_at_most_a_block_of_entries() {
    // Given: 3 blocks' worth of rows, the middle one drawing nothing
    let count = 3 * crate::blocks::CAP;
    // Not at a half block, so the move is not an append after a split.
    let middle = count / 2 + 5;
    let at = |slot: usize| f32::from(u16::try_from(slot).expect("under 65,536")) * 10.0;
    let mut boxes: Vec<_> = (0..count)
        .map(|slot| part(PAGE, &[row(at(slot))]))
        .collect();
    boxes[middle] = part(PAGE, &[]);
    let mut patched = extents(&boxes);

    // When
    painted_again(
        &mut patched,
        &mut boxes,
        (middle, part(PAGE, &[row(at(middle))])),
    );
    let put_in = patched.take_counts().moved;
    painted_again(&mut patched, &mut boxes, (middle, part(PAGE, &[])));
    let taken_out = patched.take_counts().moved;

    // Then
    assert!(put_in <= crate::blocks::CAP, "put in moved {put_in}");
    assert!(
        taken_out <= crate::blocks::CAP,
        "taken out moved {taken_out}"
    );
    assert!(
        put_in > 0 && taken_out > 0,
        "a middle entry moves a block's tail"
    );
    assert_eq!(patched, extents(&boxes));
    assert_blocks_total(&mut patched);
    assert_eq!(
        patched.take_counts().blocks,
        4,
        "three full blocks, one split by the step put in"
    );
}

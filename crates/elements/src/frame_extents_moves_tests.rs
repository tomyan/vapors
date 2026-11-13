//! The extents index's patches move at most a block of entries
//! (`blocks.rs`), whatever the frame's size.

use super::tests::{extents, painted_again, part, row};
use crate::frames::PAGE;

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
    let put_in = patched.take_moved();
    painted_again(&mut patched, &mut boxes, (middle, part(PAGE, &[])));
    let taken_out = patched.take_moved();

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
}

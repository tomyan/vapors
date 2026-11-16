use super::{Slot, holds, joined};
use crate::RowOffsets;

/// A slot holding `head` and `rows`, as `write` leaves it.
fn held(head: [u8; 16], rows: &RowOffsets) -> Slot<{ 16 + RowOffsets::CAPACITY * 8 }> {
    Slot {
        value: joined(head, rows),
        hash: 0,
        rows_none: rows.is_none(),
        encoder: 0,
    }
}

#[test]
fn a_slot_holds_its_head_and_table_and_no_other() {
    // Given
    let rowed = RowOffsets::whole(&[[0.0, 4.0]]);
    let (with_rows, empty) = (held([1; 16], &rowed), held([1; 16], &RowOffsets::NONE));

    // Then: each holds its own, not the other's, even were the hashes equal
    assert!(holds(&with_rows, (&[1; 16], &rowed)));
    assert!(holds(&empty, (&[1; 16], &RowOffsets::NONE)));
    assert!(!holds(&with_rows, (&[1; 16], &RowOffsets::NONE)));
    assert!(!holds(&empty, (&[1; 16], &rowed)));
    assert!(!holds(&empty, (&[2; 16], &RowOffsets::NONE)));
}

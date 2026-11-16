use crate::frame_extents::tests::{extents, rows};
use crate::paint_order::Step;

#[test]
#[should_panic(expected = "the extents index has no entry for paint step label 12345")]
fn a_relabel_from_a_label_the_entry_does_not_hold_is_refused() {
    // Given: ten rows, each step labelled by a walk
    let mut index = extents(&rows());

    // When: box 3 relabelled from a label it never had
    index.relabel(&[(Step::Box(3), 12_345, 999)]);
}

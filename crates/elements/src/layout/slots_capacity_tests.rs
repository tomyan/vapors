//! A placing that leaves a slot vacant grows the box list by a quarter
//! (property 8: doubled, a box being about 2.8 KB).

use super::*;

#[test]
fn a_vacated_slot_grows_the_placed_boxes_by_a_quarter() {
    // Given: a root and eight children in nine slots
    let first = place_each(vec![tree(&[
        (1, &[1, 2, 3, 4, 5, 6, 7, 8]),
        (2, &[]),
        (3, &[]),
        (4, &[]),
        (5, &[]),
        (6, &[]),
        (7, &[]),
        (8, &[]),
        (9, &[]),
    ])]);

    // When: the last child gone, eight boxes built into the nine slots
    let built = tree(&[
        (1, &[1, 2, 3, 4, 5, 6, 7]),
        (2, &[]),
        (3, &[]),
        (4, &[]),
        (5, &[]),
        (6, &[]),
        (7, &[]),
        (8, &[]),
    ]);
    assert_eq!(built.capacity(), built.len());
    let second = first.slots.place(built);

    // Then
    let (len, capacity) = (second.boxes.len(), second.boxes.capacity());
    assert_eq!(len, 9);
    assert!(capacity <= len + len / 4 + 1, "{len} boxes in {capacity}");
}

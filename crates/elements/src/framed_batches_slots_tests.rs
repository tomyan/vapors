use super::RowSlots;

#[test]
fn a_row_keeps_its_slot_while_shown_and_a_new_row_takes_one_given_back() {
    // Given: rows 7, 8 and 9 in a lane's first three slots
    let mut slots = RowSlots::default();
    let lane = slots.lane(1);
    let first: Vec<u32> = [7, 8, 9].map(|row| lane.slot(row)).to_vec();

    // When: row 8 leaves the view, and row 10 is shown
    slots.keep_only([(7, 1), (9, 1)].into_iter());
    let lane = slots.lane(1);
    let (kept, new) = ((lane.slot(7), lane.slot(9)), lane.slot(10));

    // Then
    assert_eq!(first, [0, 1, 2]);
    assert_eq!(kept, (0, 2));
    assert_eq!(new, 1);
}

#[test]
fn the_lowest_slot_given_back_is_taken_first() {
    // Given: five rows, then the second and fourth gone
    let mut slots = RowSlots::default();
    let lane = slots.lane(1);
    for row in 0..5 {
        lane.slot(row);
    }
    slots.keep_only([(0, 1), (2, 1), (4, 1)].into_iter());

    // When
    let lane = slots.lane(1);
    let taken = (lane.slot(10), lane.slot(11), lane.slot(12));

    // Then
    assert_eq!(taken, (1, 3, 5));
}

#[test]
fn a_lane_with_no_row_shown_is_dropped() {
    // Given
    let mut slots = RowSlots::default();
    slots.lane(1).slot(7);

    // When
    slots.keep_only(std::iter::empty());

    // Then
    assert_eq!(slots, RowSlots::default());
}

#[test]
fn a_row_shown_in_another_lane_gives_back_its_slot_in_this_one() {
    // Given: frame 7 a row of lane 1
    let mut slots = RowSlots::default();
    slots.lane(1).slot(7);

    // When: frame 7 is shown, as a row of lane 2
    slots.keep_only([(7, 2)].into_iter());

    // Then: lane 1 holds no slot for it, and is dropped
    assert_eq!(slots, RowSlots::default());
}

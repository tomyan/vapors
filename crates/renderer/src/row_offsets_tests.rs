use super::RowOffsets;

#[test]
fn rows_are_packed_two_to_sixteen_bytes_in_order() {
    // When
    let bytes = *RowOffsets::whole(&[[0.0, 3.0], [1.0, 24.0], [0.0, 48.0]]).bytes();

    // Then
    let float = |at: usize| f32::from_le_bytes(bytes[at * 4..at * 4 + 4].try_into().unwrap());
    let floats: Vec<f32> = (0..8).map(float).collect();
    assert_eq!(floats, [0.0, 3.0, 1.0, 24.0, 0.0, 48.0, 0.0, 0.0]);
    assert_eq!(bytes.len(), 1024);
}

#[test]
fn a_full_table_is_kept() {
    let offsets = vec![[0.0, 1.0]; RowOffsets::CAPACITY];
    assert_eq!(
        RowOffsets::whole(&offsets).bytes()[1020..],
        1.0f32.to_le_bytes()
    );
}

#[test]
#[should_panic(expected = "129 row offsets for one batch, over the table's 128")]
fn one_row_over_the_table_is_refused() {
    let _ = RowOffsets::whole(&vec![[0.0, 0.0]; RowOffsets::CAPACITY + 1]);
}

#[test]
#[should_panic(expected = "row 1's offset of [0.0, 2.5] device pixels is not whole")]
fn an_offset_that_is_not_whole_is_refused() {
    let _ = RowOffsets::whole(&[[0.0, 0.0], [0.0, 2.5]]);
}

#[test]
fn the_empty_tables_hash_is_its_bytes_hash() {
    let none = RowOffsets::NONE;
    assert_eq!(none.hash(), crate::uniform_slots::word_hash(none.bytes()));
    assert_eq!(RowOffsets::whole(&[]).hash(), none.hash());
}

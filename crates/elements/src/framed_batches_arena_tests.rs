use super::{BatchRows, RowsOfBatches};

#[test]
fn each_batch_reads_its_own_rows_numbered_as_they_first_appear() {
    // Given: a lane batch over rows 7, 7, 8 then 7; one in a frame; a
    // lane batch over row 9
    let mut rows = RowsOfBatches::default();
    rows.start_batch();
    for row in [7, 7, 8, 7] {
        rows.push_item(row);
    }
    rows.add_unrowed(1);
    rows.start_batch();
    rows.push_item(9);

    // When / Then
    let first = BatchRows {
        frames: &[7, 8],
        items: &[0, 0, 1, 0],
    };
    let last = BatchRows {
        frames: &[9],
        items: &[0],
    };
    assert_eq!(
        rows.iter().collect::<Vec<_>>(),
        [first, BatchRows::default(), last]
    );
}

#[test]
fn appended_batches_keep_their_rows() {
    // Given
    let mut rows = RowsOfBatches::default();
    rows.start_batch();
    rows.push_item(3);
    let mut more = RowsOfBatches::default();
    more.start_batch();
    more.push_item(4);
    more.push_item(5);

    // When
    rows.append(&more);

    // Then
    let second = BatchRows {
        frames: &[4, 5],
        items: &[0, 1],
    };
    assert_eq!(rows.get(1), Some(second));
}

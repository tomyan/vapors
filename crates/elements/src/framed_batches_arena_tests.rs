use super::{BatchRows, RowsOfBatches};

#[test]
fn the_batches_of_a_part_share_its_table() {
    // Given: a part whose slots hold rows 7, 8 and 9; two batches in it;
    // then a batch in one frame
    let mut rows = RowsOfBatches::default();
    let part = rows.open_part();
    rows.start_batch(part);
    rows.push_item(0);
    rows.push_item(2);
    rows.start_batch(part);
    rows.push_item(1);
    rows.close_part(part, &[7, 8, 9]);
    rows.add_unrowed(1);

    // When / Then
    let table: &[usize] = &[7, 8, 9];
    let expected = [
        BatchRows {
            frames: table,
            items: &[0, 2],
        },
        BatchRows {
            frames: table,
            items: &[1],
        },
        BatchRows::default(),
    ];
    assert_eq!(rows.iter().collect::<Vec<_>>(), expected);
}

#[test]
fn appended_batches_keep_their_parts_tables() {
    // Given
    let mut rows = RowsOfBatches::one(&[3], &[0]);
    let more = RowsOfBatches::one(&[4, 5], &[1, 0]);

    // When
    rows.append(&more);

    // Then
    let second = BatchRows {
        frames: &[4, 5],
        items: &[1, 0],
    };
    assert_eq!(rows.get(1), Some(second));
}

#[test]
fn each_part_knows_its_first_batch_after_an_append() {
    // Given: a part of two batches, then one more part appended
    let mut rows = RowsOfBatches::default();
    let part = rows.open_part();
    rows.start_batch(part);
    rows.push_item(0);
    rows.start_batch(part);
    rows.push_item(1);
    rows.close_part(part, &[7, 8]);
    rows.append(&RowsOfBatches::one(&[9], &[0]));

    // When / Then
    assert_eq!(
        (rows.first_of_part(0), rows.first_of_part(1)),
        (Some(0), Some(2))
    );
    assert_eq!((rows.part(1), rows.part(2)), (Some(0), Some(1)));
}

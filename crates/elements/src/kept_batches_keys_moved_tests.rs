//! Keys kept across a move (`kept_batches_keys.rs`): a scroll drops
//! batches at one edge and adds them at the other; rows moved by the
//! offset and rows in place both keep their keys, each key once.

use vapors_scene::{Batched, Rect};

use super::tests::{batches, keyed, quad};

/// A batch per row of `rows`, each a quad under a clip other than its
/// neighbours': what a list in a view shows.
fn rows(rows: std::ops::Range<u8>) -> Batched {
    rows_of(&rows.collect::<Vec<_>>())
}

/// As [`rows`], for the rows `rows` in order.
fn rows_of(rows: &[u8]) -> Batched {
    let clip = |at: usize| {
        at.is_multiple_of(2)
            .then(|| Rect::new(0.0, 0.0, 90.0, 90.0))
    };
    let row = |(at, &row): (usize, &u8)| (quad(f32::from(row), row), clip(at));
    batches(&rows.iter().enumerate().map(row).collect::<Vec<_>>())
}

#[test]
fn a_scroll_down_leaves_the_rows_still_in_view_their_keys() {
    // Given: rows 0 to 5 in view
    let was = keyed(rows(0..6), None);

    // When: rows 2 to 7
    let again = keyed(rows(2..8), Some(&was));

    // Then
    assert_eq!(again.keys[..4], was.keys[2..]);
    assert!(again.keys[4..].iter().all(|key| !was.keys.contains(key)));
}

#[test]
fn a_scroll_up_leaves_the_rows_still_in_view_their_keys() {
    // Given: rows 2 to 7 in view
    let was = keyed(rows(2..8), None);

    // When: rows 0 to 5
    let again = keyed(rows(0..6), Some(&was));

    // Then
    assert_eq!(again.keys[2..], was.keys[..4]);
    assert!(again.keys[..2].iter().all(|key| !was.keys.contains(key)));
}

#[test]
fn batches_matched_by_the_offset_and_by_place_from_the_end_all_keep_their_keys() {
    // Given
    let was = keyed(rows_of(&[0, 1, 2, 3, 4, 5]), None);

    // When: row 0 gone from the start, rows 1 and 2 moved up, two new
    // rows where 3 was, rows 4 and 5 in place
    let again = keyed(rows_of(&[1, 2, 50, 60, 4, 5]), Some(&was));

    // Then
    assert_eq!(again.keys[..2], was.keys[1..3]);
    assert_eq!(again.keys[4..], was.keys[4..]);
    assert!(again.keys[2..4].iter().all(|key| !was.keys.contains(key)));
}

#[test]
fn a_batch_the_offset_matched_is_not_matched_again_from_the_end() {
    // Given
    let was = keyed(rows_of(&[0, 1, 2]), None);

    // When: rows 1 and 2 moved up by the offset; row 2 again at the end,
    // where from the end it would match row 2 too
    let again = keyed(rows_of(&[1, 2, 2]), Some(&was));

    // Then: row 2's key is kept once
    assert_eq!(again.keys[..2], was.keys[1..]);
    assert!(!was.keys.contains(&again.keys[2]));
}

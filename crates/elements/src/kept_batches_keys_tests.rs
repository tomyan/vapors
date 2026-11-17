use vapors_scene::{Batched, ImageId, Item, Rect, batched};

use super::KeptBatched;
use crate::frames::PAGE;
use crate::frames_draw::BatchFrame;

const VIEW: Rect = Rect {
    left: 0.0,
    top: 0.0,
    right: 100.0,
    bottom: 100.0,
};

/// An image `id` at `top`. An image, as its batches split at each clip
/// change, which these tests use to make batches; quads and text join
/// across clips (vapors-scene `Run::clipped_by_table`).
pub(super) fn quad(top: f32, id: u8) -> Item {
    Item::Image {
        image: ImageId(u32::from(id)),
        rect: Rect::new(10.0, top, 20.0, top + 10.0),
    }
}

/// A quad at `top`.
fn filled(top: f32) -> Item {
    let rect = Rect::new(10.0, top, 20.0, top + 10.0);
    Item::Quad(vapors_scene::Quad::fill(
        rect,
        vapors_scene::Color::from_rgba8(1, 2, 3, 255),
    ))
}

/// The batches of `items`, each under its clip.
pub(super) fn batches(items: &[(Item, Option<Rect>)]) -> Batched {
    batched(items.iter().map(|(item, clip)| (item, *clip)), VIEW)
}

/// `batched` on the page, keyed against `was`.
pub(super) fn keyed(batched: Batched, was: Option<&KeptBatched>) -> KeptBatched {
    let count = batched.batches.len();
    let mut rows = crate::framed_batches::RowsOfBatches::default();
    rows.add_unrowed(count);
    let framed = crate::framed_batches::FramedBatched {
        frames: vec![BatchFrame::frame(PAGE); count],
        rows,
        batched,
    };
    KeptBatched::keyed(framed, was)
}

/// Three batches: a quad unclipped, one clipped, one unclipped again.
fn three(middle: Item) -> Batched {
    let clip = Some(Rect::new(0.0, 0.0, 50.0, 50.0));
    batches(&[(quad(0.0, 1), None), (middle, clip), (quad(20.0, 1), None)])
}

#[test]
fn batches_made_again_with_equal_runs_keep_their_keys() {
    // Given
    let was = keyed(three(quad(10.0, 1)), None);

    // When
    let again = keyed(three(quad(10.0, 1)), Some(&was));

    // Then
    assert_eq!(again.keys, was.keys);
}

#[test]
fn a_batch_whose_run_changed_gets_a_new_key_and_the_others_keep_theirs() {
    // Given
    let was = keyed(three(quad(10.0, 1)), None);

    // When: the middle quad's colour changed
    let again = keyed(three(quad(10.0, 9)), Some(&was));

    // Then
    assert_eq!((again.keys[0], again.keys[2]), (was.keys[0], was.keys[2]));
    assert_ne!(again.keys[1], was.keys[1]);
}

#[test]
fn batches_made_without_earlier_ones_each_get_a_key_of_their_own() {
    // When
    let first = keyed(three(quad(10.0, 1)), None);
    let second = keyed(three(quad(10.0, 1)), None);

    // Then
    let mut keys = [first.keys, second.keys].concat();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), 6);
}

#[test]
fn a_batch_at_a_place_the_earlier_batches_did_not_reach_gets_a_new_key() {
    // Given: one batch
    let was = keyed(batches(&[(quad(0.0, 1), None)]), None);

    // When: the same batch, then one more
    let clip = Some(Rect::new(0.0, 0.0, 50.0, 50.0));
    let more = batches(&[(quad(0.0, 1), None), (quad(10.0, 1), clip)]);
    let again = keyed(more, Some(&was));

    // Then
    assert_eq!(again.keys[0], was.keys[0]);
    assert_eq!(again.keys.len(), 2);
    assert_ne!(again.keys[1], was.keys[0]);
}

#[test]
fn a_batch_put_in_leaves_the_batches_after_it_their_keys() {
    // Given: two batches
    let clip = Some(Rect::new(0.0, 0.0, 50.0, 50.0));
    let was = keyed(
        batches(&[(quad(0.0, 1), None), (quad(20.0, 1), clip)]),
        None,
    );

    // When: a batch put in between
    let between = (quad(10.0, 1), Some(Rect::new(0.0, 0.0, 60.0, 60.0)));
    let now = batches(&[(quad(0.0, 1), None), between, (quad(20.0, 1), clip)]);
    let again = keyed(now, Some(&was));

    // Then
    assert_eq!((again.keys[0], again.keys[2]), (was.keys[0], was.keys[1]));
    assert!(!was.keys.contains(&again.keys[1]));
}

#[test]
fn a_batch_taken_out_leaves_the_batches_after_it_their_keys() {
    // Given: three batches
    let was = keyed(three(quad(10.0, 1)), None);

    // When: the middle one taken out
    let again = keyed(
        batches(&[(quad(0.0, 1), None), (quad(20.0, 1), None)]),
        Some(&was),
    );

    // Then: the two left merge into one batch, whose run is new
    assert_eq!(again.batched.batches.len(), 1);
    assert!(!was.keys.contains(&again.keys[0]));
}

#[test]
fn a_run_equal_to_one_matched_from_the_start_is_not_matched_again_from_the_end() {
    // Given: one batch
    let was = keyed(batches(&[(quad(0.0, 1), None)]), None);

    // When: the same batch twice, under two clips
    let clip = Some(Rect::new(0.0, 0.0, 50.0, 50.0));
    let again = keyed(
        batches(&[(quad(0.0, 1), None), (quad(0.0, 1), clip)]),
        Some(&was),
    );

    // Then: the key is kept once
    assert_eq!(again.keys[0], was.keys[0]);
    assert_ne!(again.keys[1], was.keys[0]);
}

#[test]
fn a_batch_whose_edge_went_from_minus_zero_to_zero_gets_a_new_key() {
    // Given: the middle batch a quad, its top at -0.0
    let was = keyed(three(filled(-0.0)), None);

    // When: at 0.0, a run equal under == with other bytes
    let again = keyed(three(filled(0.0)), Some(&was));

    // Then
    assert_eq!((again.keys[0], again.keys[2]), (was.keys[0], was.keys[2]));
    assert_ne!(again.keys[1], was.keys[1]);
}

/// One batch of two quads, each in the row `items` names, in rows 7
/// and 8, keyed against `was`.
fn in_rows(items: [u32; 2], was: Option<&KeptBatched>) -> KeptBatched {
    in_row_frames(items, [7, 8], was)
}

/// As [`in_rows`], the rows' frames `row_frames`.
fn in_row_frames(
    items: [u32; 2],
    row_frames: [usize; 2],
    was: Option<&KeptBatched>,
) -> KeptBatched {
    let batched = batches(&[(quad(0.0, 1), None), (quad(20.0, 1), None)]);
    let rows = crate::framed_batches::RowsOfBatches::one(&row_frames, &items);
    let framed = crate::framed_batches::FramedBatched {
        frames: vec![BatchFrame::lane(3)],
        rows,
        batched,
    };
    KeptBatched::keyed(framed, was)
}

#[test]
fn a_batch_whose_items_moved_to_other_rows_gets_a_new_key() {
    // Given: the quads in rows 0 and 1
    let was = in_rows([0, 1], None);

    // When: both in row 0, the same run to the bit
    let again = in_rows([0, 0], Some(&was));

    // Then: its bytes, holding each item's row, are others
    assert_ne!(again.keys, was.keys);
    assert_eq!(in_rows([0, 1], Some(&was)).keys, was.keys);
}

#[test]
fn a_batch_whose_items_are_in_other_rows_frames_at_the_same_indexes_keeps_its_key() {
    // Given: the quads in rows 0 and 1, frames 7 and 8
    let was = in_rows([0, 1], None);

    // When: a scroll by two rows puts the same items in frames 9 and 10
    let again = in_row_frames([0, 1], [9, 10], Some(&was));

    // Then: its bytes, holding the indexes, are the same; the painter
    // reads the new frames' offsets
    assert_eq!(again.keys, was.keys);
}

use vapors_scene::{Batched, Color, Item, Quad, Rect, batched};

use super::KeptBatched;

const VIEW: Rect = Rect {
    left: 0.0,
    top: 0.0,
    right: 100.0,
    bottom: 100.0,
};

fn quad(top: f32, red: u8) -> Item {
    let rect = Rect::new(10.0, top, 20.0, top + 10.0);
    Item::Quad(Quad::fill(rect, Color::from_rgba8(red, 2, 3, 255)))
}

/// The batches of `items`, each under its clip.
fn batches(items: &[(Item, Option<Rect>)]) -> Batched {
    batched(items.iter().map(|(item, clip)| (item, *clip)), VIEW)
}

/// Three batches: a quad unclipped, one clipped, one unclipped again.
fn three(middle: Item) -> Batched {
    let clip = Some(Rect::new(0.0, 0.0, 50.0, 50.0));
    batches(&[(quad(0.0, 1), None), (middle, clip), (quad(20.0, 1), None)])
}

#[test]
fn batches_made_again_with_equal_runs_keep_their_keys() {
    // Given
    let was = KeptBatched::keyed(three(quad(10.0, 1)), None);

    // When
    let again = KeptBatched::keyed(three(quad(10.0, 1)), Some(&was));

    // Then
    assert_eq!(again.keys, was.keys);
}

#[test]
fn a_batch_whose_run_changed_gets_a_new_key_and_the_others_keep_theirs() {
    // Given
    let was = KeptBatched::keyed(three(quad(10.0, 1)), None);

    // When: the middle quad's colour changed
    let again = KeptBatched::keyed(three(quad(10.0, 9)), Some(&was));

    // Then
    assert_eq!((again.keys[0], again.keys[2]), (was.keys[0], was.keys[2]));
    assert_ne!(again.keys[1], was.keys[1]);
}

#[test]
fn batches_made_without_earlier_ones_each_get_a_key_of_their_own() {
    // When
    let first = KeptBatched::keyed(three(quad(10.0, 1)), None);
    let second = KeptBatched::keyed(three(quad(10.0, 1)), None);

    // Then
    let mut keys = [first.keys, second.keys].concat();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), 6);
}

#[test]
fn a_batch_at_a_place_the_earlier_batches_did_not_reach_gets_a_new_key() {
    // Given: one batch
    let was = KeptBatched::keyed(batches(&[(quad(0.0, 1), None)]), None);

    // When: the same batch, then one more
    let clip = Some(Rect::new(0.0, 0.0, 50.0, 50.0));
    let more = batches(&[(quad(0.0, 1), None), (quad(10.0, 1), clip)]);
    let again = KeptBatched::keyed(more, Some(&was));

    // Then
    assert_eq!(again.keys[0], was.keys[0]);
    assert_eq!(again.keys.len(), 2);
    assert_ne!(again.keys[1], was.keys[0]);
}

#[test]
fn a_batch_put_in_leaves_the_batches_after_it_their_keys() {
    // Given: two batches
    let clip = Some(Rect::new(0.0, 0.0, 50.0, 50.0));
    let was = KeptBatched::keyed(
        batches(&[(quad(0.0, 1), None), (quad(20.0, 1), clip)]),
        None,
    );

    // When: a batch put in between
    let between = (quad(10.0, 1), Some(Rect::new(0.0, 0.0, 60.0, 60.0)));
    let now = batches(&[(quad(0.0, 1), None), between, (quad(20.0, 1), clip)]);
    let again = KeptBatched::keyed(now, Some(&was));

    // Then
    assert_eq!((again.keys[0], again.keys[2]), (was.keys[0], was.keys[1]));
    assert!(!was.keys.contains(&again.keys[1]));
}

#[test]
fn a_batch_taken_out_leaves_the_batches_after_it_their_keys() {
    // Given: three batches
    let was = KeptBatched::keyed(three(quad(10.0, 1)), None);

    // When: the middle one taken out
    let again = KeptBatched::keyed(
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
    let was = KeptBatched::keyed(batches(&[(quad(0.0, 1), None)]), None);

    // When: the same batch twice, under two clips
    let clip = Some(Rect::new(0.0, 0.0, 50.0, 50.0));
    let again = KeptBatched::keyed(
        batches(&[(quad(0.0, 1), None), (quad(0.0, 1), clip)]),
        Some(&was),
    );

    // Then: the key is kept once
    assert_eq!(again.keys[0], was.keys[0]);
    assert_ne!(again.keys[1], was.keys[0]);
}

#[test]
fn a_batch_whose_edge_went_from_minus_zero_to_zero_gets_a_new_key() {
    // Given: the middle quad's top at -0.0
    let was = KeptBatched::keyed(three(quad(-0.0, 1)), None);

    // When: at 0.0, a run equal under == with other bytes
    let again = KeptBatched::keyed(three(quad(0.0, 1)), Some(&was));

    // Then
    assert_eq!((again.keys[0], again.keys[2]), (was.keys[0], was.keys[2]));
    assert_ne!(again.keys[1], was.keys[1]);
}

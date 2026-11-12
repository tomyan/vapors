//! Each kept batch's key (IU4 PR 2, `kept_batches_keys.rs`): after an
//! edit, a batch made again keeps a key only where the batch it is
//! matched with has an equal run; a class toggle gives new keys
//! only to the batches holding the row's items.

mod support;

use support::rows_page::{EDITS, Edit, RowsPage, rows_page, rows_viewport};
use vapors_elements::KeptBatched;
use vapors_scene::Rect;

/// The rows page's viewport in device pixels.
const VIEW: Rect = Rect {
    left: 0.0,
    top: 0.0,
    right: 640.0,
    bottom: 400.0,
};

/// `page` updated and its batches for `VIEW`, with their keys.
fn drawn(page: &RowsPage) -> KeptBatched {
    let mut doc = page.doc.borrow_mut();
    let painted = doc.update(rows_viewport()).expect("updates");
    painted.kept_batches(VIEW).clone()
}

/// The places of `now` whose key is new; panics, naming `case`, when a
/// key kept from `was` is on a batch whose run is not the one it had.
fn new_keys(was: &KeptBatched, now: &KeptBatched, case: &str) -> Vec<usize> {
    let places = 0..now.keys.len();
    let kept_from = |at: usize| was.keys.iter().position(|&key| key == now.keys[at]);
    for (at, from) in places.clone().filter_map(|at| Some((at, kept_from(at)?))) {
        let (run, had) = (&now.batched.batches[at].run, &was.batched.batches[from].run);
        assert_eq!(run, had, "{case}: batch {at} kept batch {from}'s key");
    }
    places.filter(|&at| kept_from(at).is_none()).collect()
}

#[test]
fn after_each_edit_each_key_kept_is_on_a_batch_with_the_run_it_had() {
    for edit in EDITS {
        // Given: ten rows drawn, the middle one in view
        let page = rows_page(10);
        let was = drawn(&page);

        // When
        page.edit(edit);
        let now = drawn(&page);

        // Then
        let new = new_keys(&was, &now, &format!("{edit:?}"));
        assert!(new.len() < now.keys.len(), "{edit:?} kept no key");
    }
}

#[test]
fn a_class_toggle_gives_new_keys_only_to_the_rows_batches() {
    // Given
    let page = rows_page(10);
    let was = drawn(&page);

    // When: the middle row's background
    page.edit(Edit::ToggleClass);
    let now = drawn(&page);

    // Then: the row's text batch, split by its new background into a
    // text, the background and a text, and the batch its last text
    // shares with the next row's first (its texts were laid out again)
    let new = new_keys(&was, &now, "class");
    assert_eq!(new.len(), 4, "{new:?}");
}

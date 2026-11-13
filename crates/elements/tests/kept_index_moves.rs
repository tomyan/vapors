//! The two ordered indexes paint patches, the drawing steps
//! (`drawn_steps.rs`) and each scroll frame's extents (`frame_extents.rs`),
//! are kept in blocks (`blocks.rs`): a step that starts or stops drawing
//! moves at most a block's entries in each, whatever the page's size.
//! Before, each was a `Vec` whose middle insert moved half the page's
//! entries: 13.6 instructions per row on `--scaling Class`'s add frames
//! (`docs/perf/class-add-paint-2026-11-13.md`).

mod support;

use support::rows_page::{Edit, rows_page, rows_viewport};
use vapors_elements::UpdateCounts;

/// The most entries one index moves per patch; two indexes are patched.
const BLOCK: usize = UpdateCounts::INDEX_BLOCK;

/// Toggles the middle row's class on `rows` rows, once each way, and
/// holds the entries moved under two blocks each time, by a patch (no
/// order walk, no rebuild of the extents index) that moved something.
fn assert_moves_at_most_two_blocks(rows: usize) {
    // Given
    let page = rows_page(rows);

    for kind in ["added", "removed"] {
        // When: the middle row's background comes and goes
        page.edit(Edit::ToggleClass);
        let counts = {
            let mut doc = page.doc.borrow_mut();
            doc.update(rows_viewport()).expect("updates");
            doc.update_counts()
        };

        // Then
        let case = format!("{rows} rows, class {kind}");
        assert_eq!(
            (counts.order_walks, counts.index_rebuilds),
            (0, 0),
            "{case}: the indexes were made afresh, not patched (paint_partial.rs \
             drawn_updated, frame_extents.rs REBUILD); the count of entries moved says \
             nothing then"
        );
        let moved = counts.index_entries_moved;
        assert!(
            moved > 0,
            "{case}: the patches moved no entry, so the step did not change the indexes \
             (drawn_steps.rs, frame_extents_patch.rs)"
        );
        let most = 2 * BLOCK + counts.index_blocks;
        assert!(
            moved <= most,
            "{case}: the indexes moved {moved} entries, over two blocks of {BLOCK} and \
             their {} block pointers; a patch moved an index's tail (blocks.rs, \
             drawn_steps.rs, frame_extents_patch.rs)",
            counts.index_blocks
        );
    }
}

// One test per size: a debug build paints the whole page again after
// every update to check the kept paint, so the 10,000-row case alone
// takes most of a test's deadline under load.
#[test]
fn a_class_toggled_on_1000_rows_moves_at_most_a_block_of_each_indexs_entries() {
    assert_moves_at_most_two_blocks(1000);
}

#[test]
fn a_class_toggled_on_3000_rows_moves_at_most_a_block_of_each_indexs_entries() {
    assert_moves_at_most_two_blocks(3000);
}

#[test]
fn a_class_toggled_on_10_000_rows_moves_at_most_a_block_of_each_indexs_entries() {
    assert_moves_at_most_two_blocks(10_000);
}

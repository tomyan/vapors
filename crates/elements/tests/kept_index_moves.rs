//! The two ordered indexes paint patches, the drawing steps
//! (`drawn_steps.rs`) and each scroll frame's extents (`frame_extents.rs`),
//! are kept in blocks (`blocks.rs`): a step that starts or stops drawing
//! moves at most a block's entries in each, whatever the page's size.
//! Before, each was a `Vec` whose middle insert moved half the page's
//! entries: 13.6 instructions per row on `--scaling Class`'s add frames
//! (`docs/perf/class-add-paint-2026-11-13.md`).

mod support;

use support::rows_page::{Edit, rows_page, rows_viewport};

/// `blocks.rs` `CAP`, the most entries one index moves per patch; two
/// indexes are patched.
const BLOCK: usize = 128;

#[test]
fn a_class_added_or_removed_moves_at_most_a_block_of_each_indexs_entries() {
    for rows in [1000, 3000, 10_000] {
        // Given
        let page = rows_page(rows);

        for kind in ["added", "removed"] {
            // When: the middle row's background comes and goes
            page.edit(Edit::ToggleClass);
            let moved = {
                let mut doc = page.doc.borrow_mut();
                doc.update(rows_viewport()).expect("updates");
                doc.update_counts().index_entries_moved
            };

            // Then
            assert!(
                moved <= 2 * BLOCK,
                "{rows} rows, class {kind}: the indexes moved {moved} entries, over two blocks \
                 of {BLOCK}; a patch moved an index's tail (blocks.rs, drawn_steps.rs, \
                 frame_extents_patch.rs)"
            );
        }
    }
}

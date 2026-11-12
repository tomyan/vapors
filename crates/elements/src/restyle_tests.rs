//! What the style phase keeps between updates is sized by the last
//! edit, not by the page (docs/plans/perf-finish-line.md, property 1).

use vapors_dom::Dom;

use crate::test_rows::{ROWS, mounted, viewport};

#[test]
fn an_edit_after_a_mount_iterates_a_restyled_set_sized_by_the_edit() {
    // Given: every row restyled by the mount
    let (mut doc, rows) = mounted(false);

    // When: one row's class changes
    doc.set_attribute(rows[0], "class", "on");
    doc.update(viewport()).expect("updates");

    // Then: the set `rechecked` iterates holds the edit, at the edit's capacity
    let restyled = &doc.kept.as_ref().expect("kept").restyled_nodes;
    assert_eq!(restyled.len(), 1, "{restyled:?}");
    assert!(
        restyled.capacity() < ROWS / 10,
        "the restyled set has capacity {} after a one-row edit on {ROWS} rows: it keeps the mount's, and iterating it is O(rows) per update (restyle.rs, Kept::forget_restyled)",
        restyled.capacity()
    );
}

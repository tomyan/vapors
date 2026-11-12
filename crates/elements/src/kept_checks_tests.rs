//! The listener marks the style phase checks again are sized by the
//! last edit, not by the page (docs/plans/perf-finish-line.md,
//! property 1).

use crate::test_rows::{ROWS, mounted};

#[test]
fn the_listener_marks_of_a_mount_are_not_kept_at_its_capacity() {
    // Given: every row gets a listener before the mount
    // When: the mount's update runs (in `mounted`)
    let (doc, _) = mounted(true);

    // Then: the marks `touched` iterates on the next update are empty, at no capacity
    let marked = &doc.checks.marked;
    assert!(marked.is_empty(), "{marked:?}");
    assert!(
        marked.capacity() < ROWS / 10,
        "the listener marks have capacity {} after mounting {ROWS} listened rows: they keep the mount's, and iterating them is O(rows) per update (kept_checks.rs, KeptChecks::checked)",
        marked.capacity()
    );
}

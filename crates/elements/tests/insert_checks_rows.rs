//! The nodes the style phase checks again after a row is inserted
//! before the middle row of the rows page (`support::rows_page`), and
//! after it is removed: the same count at any size. The body, whose
//! children changed, is checked without reading them, since the one
//! check that reads a text child (right-to-left text) is the text
//! node's own (`check.rs`).

mod support;

use support::rows_page::{row_before, rows_page, rows_viewport};
use vapors_dom::Dom;

/// The body, and the row's four elements and three text nodes.
const INSERTED: usize = 8;
/// The body alone.
const REMOVED: usize = 1;

/// The nodes checked by the update after a row is inserted before the
/// middle row, and by the update after that row is removed.
fn checked_by_middle_insert_and_remove(rows: usize) -> (usize, usize) {
    let page = rows_page(rows);
    let mut doc = page.doc.borrow_mut();
    let middle = page.rows[rows / 2].element;
    let added = row_before(&mut doc, rows, Some(middle)).element;
    doc.update(rows_viewport()).expect("updates");
    let inserted = doc.update_counts().nodes_checked;
    doc.remove(added);
    doc.update(rows_viewport()).expect("updates");
    (inserted, doc.update_counts().nodes_checked)
}

#[test]
fn a_middle_insert_and_its_removal_check_the_same_nodes_at_any_size() {
    // Given / When
    let small = checked_by_middle_insert_and_remove(100);
    let large = checked_by_middle_insert_and_remove(400);

    // Then
    assert_eq!(
        (small, large),
        ((INSERTED, REMOVED), (INSERTED, REMOVED)),
        "nodes checked by (insert, remove) at 100 and 400 rows"
    );
}

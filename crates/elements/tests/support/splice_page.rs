//! A rows page for the order splice tests (`order_splice.rs`,
//! `order_splice_declined.rs`): rows under the body with the classes a
//! splice declines on, a row inserted before another, and the order
//! counts of an update.

use vapors_dom::{Dom, NodeId};
use vapors_elements::UpdateCounts;

use super::oracle::assert_matches_full;
use super::rows_page::{RowsPage, row_before, rows_page, rows_viewport};

/// The row count of the acceptance case (`docs/plans/splice.md`'s PR
/// stack table, 3000): `VAPORS_SPARSE_ROWS` when set, as
/// `sparse_updates.rs` reads it, else 1000, since a debug update of
/// 3000 rows with its checks takes about 15 s on the dev machine
/// (2026-11-12) against the 30 s deadline of `.config/nextest.toml`.
pub fn many_rows() -> usize {
    let Ok(rows) = std::env::var("VAPORS_SPARSE_ROWS") else {
        return 1000;
    };
    rows.parse()
        .unwrap_or_else(|_| panic!("VAPORS_SPARSE_ROWS={rows} is not a row count"))
}

/// Inside a row's box, right of its cells.
pub const ROW_X: f32 = 630.0;

/// Rows a splice declines on: positioned, scrolling, or with a z-index
/// on a cell (a flex item, so the z-index applies).
const DECLINED: &str = ".row.positioned { position: relative; } \
    .row.scrolling { overflow: scroll; } \
    .row.raised > .value { z-index: 1; }";

pub fn page(count: usize) -> RowsPage {
    let page = rows_page(count);
    page.doc
        .borrow_mut()
        .add_stylesheet("declined.css", DECLINED)
        .expect("parses");
    updated(&page);
    page
}

pub fn updated(page: &RowsPage) -> UpdateCounts {
    let mut doc = page.doc.borrow_mut();
    doc.update(rows_viewport()).expect("updates");
    doc.update_counts()
}

/// A new row with `class`, inserted before row `before` or appended.
pub fn inserted(page: &RowsPage, before: Option<usize>, class: &str) -> NodeId {
    let mut doc = page.doc.borrow_mut();
    let before = before.map(|index| page.rows[index].element);
    let row = row_before(&mut doc, 9999, before).element;
    if !class.is_empty() {
        doc.set_attribute(row, "class", &format!("row {class}"));
    }
    row
}

/// (order walks, chain walks, splices, declined) of `counts`.
pub fn order(counts: UpdateCounts) -> (usize, usize, usize, usize) {
    (
        counts.order_walks,
        counts.chain_walks,
        counts.order_splices,
        counts.splices_declined,
    )
}

pub const SPLICED: (usize, usize, usize, usize) = (0, 0, 1, 0);
pub const DECLINED_WALK: (usize, usize, usize, usize) = (1, 1, 0, 1);

pub fn matches_full(page: &RowsPage, context: &str) {
    assert_matches_full(&page.doc, rows_viewport(), context);
}

//! The rows page (`rows_page.rs`) with every seventh row
//! `position: relative; z-index: 1`, an absolutely positioned box and a
//! fixed footer, so paint order is not tree order
//! (`docs/plans/flat-scene.md` section 6, W6), and the edits made to it.
//! `crates/perf` times the same page, including this file by path.
//! `position: sticky` is refused by vapors (`style/position.rs`), so the
//! page has no sticky header.

use vapors_dom::{Dom, NodeId};

use super::basics::element;
use super::rows_page::{RowsPage, row_before, rows_viewport};

/// Every this many rows, from the first, is positioned.
pub const EVERY: usize = 7;

const IN_FLOW_CSS: &str = ".row.positioned { position: relative; z-index: 1; }";

const OUT_OF_FLOW_CSS: &str = ".footer { position: fixed; left: 0; bottom: 0; width: 640px; \
    height: 24px; background-color: #fff; } \
    .overlay { position: absolute; top: 96px; left: 320px; width: 200px; height: 24px; \
    background-color: #ffe; }";

/// The whole W6 page: [`position_rows`] then [`add_out_of_flow`].
pub fn make_positioned(page: &RowsPage) {
    position_rows(page);
    add_out_of_flow(page);
}

/// Makes every seventh row of `page` positioned, and updates once.
pub fn position_rows(page: &RowsPage) {
    let mut doc = page.doc.borrow_mut();
    for row in page.rows.iter().step_by(EVERY) {
        doc.set_attribute(row.element, "class", "row positioned");
    }
    doc.add_stylesheet("positioned.css", IN_FLOW_CSS)
        .expect("parses");
    doc.update(rows_viewport()).expect("updates");
}

/// Adds the absolutely positioned box and the fixed footer under the
/// body, and updates once.
pub fn add_out_of_flow(page: &RowsPage) {
    let mut doc = page.doc.borrow_mut();
    let body = doc.body();
    element(&mut doc, body, "div", "overlay");
    element(&mut doc, body, "div", "footer");
    doc.add_stylesheet("out_of_flow.css", OUT_OF_FLOW_CSS)
        .expect("parses");
    doc.update(rows_viewport()).expect("updates");
}

/// The positioned row at or before the middle row.
pub fn middle_positioned(page: &RowsPage) -> NodeId {
    page.rows[page.rows.len() / 2 / EVERY * EVERY].element
}

/// The middle positioned row's `marked` class added, or removed when it
/// has it: its background.
///
/// # Panics
/// The row is not positioned: the edit would make a plain row
/// positioned and time something else.
pub fn toggle_positioned_marked(page: &RowsPage) {
    let row = middle_positioned(page);
    let mut doc = page.doc.borrow_mut();
    let class = match doc.attribute(row, "class") {
        Some("row positioned marked") => "row positioned",
        Some("row positioned") => "row positioned marked",
        other => panic!("the middle positioned row has class {other:?} (positioned_rows.rs)"),
    };
    doc.set_attribute(row, "class", class);
}

/// A positioned row numbered `index` put in before the middle row: a
/// box with a stacking of its own.
pub fn insert_positioned(page: &RowsPage, index: usize) -> NodeId {
    let middle = page.rows[page.rows.len() / 2].element;
    let mut doc = page.doc.borrow_mut();
    let added = row_before(&mut doc, index, Some(middle)).element;
    doc.set_attribute(added, "class", "row positioned");
    added
}

//! The rows page (`rows_page.rs`) with every seventh row
//! `position: relative; z-index: 1` and a fixed footer, so paint order
//! is not tree order (`docs/plans/flat-scene.md` section 6, W6), and
//! the edits made to it. `crates/perf` times the same page, including
//! this file by path.

use vapors_dom::{Dom, NodeId};

use super::basics::element;
use super::rows_page::{RowsPage, rows_viewport};

/// Every this many rows, from the first, is positioned.
pub const EVERY: usize = 7;

const CSS: &str = ".row.positioned { position: relative; z-index: 1; } \
    .footer { position: fixed; left: 0; bottom: 0; width: 640px; height: 24px; \
    background-color: #fff; }";

/// Makes every seventh row of `page` positioned, adds the fixed footer,
/// and updates once.
pub fn make_positioned(page: &RowsPage) {
    let mut doc = page.doc.borrow_mut();
    for row in page.rows.iter().step_by(EVERY) {
        doc.set_attribute(row.element, "class", "row positioned");
    }
    let body = doc.body();
    element(&mut doc, body, "div", "footer");
    doc.add_stylesheet("positioned.css", CSS).expect("parses");
    doc.update(rows_viewport()).expect("updates");
}

/// The positioned row at or before the middle row.
pub fn middle_positioned(page: &RowsPage) -> NodeId {
    page.rows[page.rows.len() / 2 / EVERY * EVERY].element
}

/// The middle positioned row's `marked` class added, or removed when it
/// has it: its background.
pub fn toggle_positioned_marked(page: &RowsPage) {
    let row = middle_positioned(page);
    let mut doc = page.doc.borrow_mut();
    let marked = doc.attribute(row, "class") == Some("row positioned marked");
    let class = if marked {
        "row positioned"
    } else {
        "row positioned marked"
    };
    doc.set_attribute(row, "class", class);
}

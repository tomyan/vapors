//! The elements restyled by a middle insert on the rows workload
//! (`support::rows_page`), with and without a structural rule: pinned
//! at 1000 rows, and at the count in `VAPORS_SPARSE_ROWS` (3000 for
//! the record in splice.md (b2)) when it asks, as `sparse_updates.rs`.

mod support;

use support::rows_page::{row_before, rows_page, rows_viewport};
use vapors_dom::Dom;

/// A row's elements: the row, its label, value and button.
const ROW: usize = 4;

/// The elements restyled by one update after a row is inserted before
/// the middle row, under `extra_css` added to the workload's sheet.
fn restyled_by_middle_insert(rows: usize, extra_css: &str) -> usize {
    let page = rows_page(rows);
    let mut doc = page.doc.borrow_mut();
    if !extra_css.is_empty() {
        doc.add_stylesheet("extra.css", extra_css).expect("parses");
        doc.update(rows_viewport()).expect("updates");
    }
    row_before(&mut doc, rows, Some(page.rows[rows / 2].element));
    doc.update(rows_viewport()).expect("updates");
    doc.elements_restyled()
}

fn assert_middle_insert_counts(rows: usize) {
    // When
    let plain = restyled_by_middle_insert(rows, "");
    let next = restyled_by_middle_insert(rows, ".row + .row { margin-top: 1px; }");
    let odd = restyled_by_middle_insert(rows, ".row:nth-child(odd) { color: red; }");

    // Then: the new row; and the next row; and every row after, changed, with its children
    assert_eq!(
        (plain, next, odd),
        (ROW, ROW + 1, ROW + (rows - rows / 2) * ROW),
        "elements restyled by a middle insert at {rows} rows (plain, `+`, `:nth-child(odd)`)"
    );
}

#[test]
fn a_middle_insert_at_a_thousand_rows_restyles_only_what_the_sheets_reach() {
    assert_middle_insert_counts(1000);
}

/// The record: `VAPORS_SPARSE_ROWS=3000 cargo nextest run -p vapors-elements --test insert_restyle_rows`.
#[test]
fn a_middle_insert_at_the_requested_rows_restyles_only_what_the_sheets_reach() {
    let Some(rows) = std::env::var("VAPORS_SPARSE_ROWS").ok() else {
        return;
    };
    let rows: usize = rows
        .parse()
        .unwrap_or_else(|_| panic!("VAPORS_SPARSE_ROWS={rows:?} is not a row count"));
    assert_middle_insert_counts(rows);
}

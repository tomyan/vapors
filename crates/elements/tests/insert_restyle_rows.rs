//! The elements restyled by a middle insert on the rows workload
//! (`support::rows_page`), with and without a structural rule: pinned
//! at 1000 rows. The 3000-row counts are recorded in splice.md (b2),
//! from one run of `assert_middle_insert_counts(3000)`, 22 s in debug
//! under load: too slow to run on every `cargo xtask ci`.

mod support;

use support::rows_page::{RowsPage, row_before, rows_page, rows_viewport};

/// A row's elements: the row, its label, value and button.
const ROW: usize = 4;

/// The elements restyled by one update after a row is inserted before
/// the middle row, under `extra_css` added to the sheets so far.
fn restyled_by_middle_insert(page: &RowsPage, extra_css: &str) -> usize {
    let mut doc = page.doc.borrow_mut();
    if !extra_css.is_empty() {
        doc.add_stylesheet(extra_css, extra_css).expect("parses");
        doc.update(rows_viewport()).expect("updates");
    }
    let rows = page.rows.len();
    row_before(&mut doc, rows, Some(page.rows[rows / 2].element));
    doc.update(rows_viewport()).expect("updates");
    doc.elements_restyled()
}

/// One page, the sheets added in turn: each insert goes before the
/// original middle row, so the rows after it stay `rows - rows / 2`.
fn assert_middle_insert_counts(rows: usize) {
    // Given
    let page = rows_page(rows);

    // When
    let plain = restyled_by_middle_insert(&page, "");
    let next = restyled_by_middle_insert(&page, ".row + .row { margin-top: 1px; }");
    let odd = restyled_by_middle_insert(&page, ".row:nth-child(odd) { color: red; }");

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

//! Moved slots shift their kept paint in place (IU3 3c-3e,
//! `docs/plans/iu3.md`): at 3000 rows, an edit that resizes a row near
//! the top paints only the boxes it built again and the boxes whose
//! size changed, and a scroll paints none; the rows that only moved
//! are shifted. A moved box whose shift is not exact (a fractional
//! position) is painted again, never shifted on a guess. A debug build
//! checks every shifted paint against a fresh one inside `update`
//! (`kept_paint.rs`).

mod support;

use support::oracle::assert_matches_full_without_repeat;
use support::rows_page::{RowsPage, rows_page, rows_viewport};
use support::scroll_list::{at, list, scroll};
use vapors_elements::Document;

const ROWS: usize = 3000;

/// A row is 7 boxes: the row, the label, its text, the value, its
/// text, the button, its text.
const BOXES_PER_ROW: usize = 7;

/// Boxes painted, boxes shifted.
fn painted(doc: &Document) -> (usize, usize) {
    let counts = doc.update_counts();
    (counts.boxes_painted, counts.boxes_shifted)
}

fn updated(page: &RowsPage) -> (usize, usize) {
    let mut doc = page.doc.borrow_mut();
    doc.update(rows_viewport()).expect("updates");
    painted(&doc)
}

/// Runs `edit`, updates, and checks the result against a full update
/// (the oracle), for the 3000-row tests, with two debug checks relaxed
/// because each costs seconds there:
/// - The debug layout check (`Document::skip_layout_check`) is skipped
///   from here on for this page's updates; the rows build and anything
///   before this call keep it. The oracle compares every element's box
///   with a fresh layout's: a layout error that leaves the counters and
///   the placement unchanged fails it (shown in vapors #233 by moving
///   the box of the last slot laid out).
/// - The oracle runs no repeat update per snapshot
///   (`assert_matches_full_without_repeat`).
///
/// Retire both once `[profile.test] opt-level = 1` (the workspace
/// `Cargo.toml`) is in: the three tests then ran in 3.1, 3.2 and 2.5 s
/// alone with the checks on, against 14.7, 14.6 and 15.3 s at opt-level 0
/// (measured 2026-11-12 at load ~12); or when the layout check is
/// cheaper. The skip and the oracle are one call, so one cannot be
/// removed without the other.
fn updated_and_matched_to_full(
    page: &RowsPage,
    context: &str,
    edit: impl FnOnce(&RowsPage),
) -> (usize, usize) {
    page.doc.borrow_mut().skip_layout_check();
    edit(page);
    let work = updated(page);
    assert_matches_full_without_repeat(&page.doc, rows_viewport(), context);
    work
}

#[test]
fn resizing_a_row_near_the_top_paints_its_boxes_and_the_body_and_shifts_the_rows_after() {
    // Given
    let page = rows_page(ROWS);
    page.rows_resizable();

    // When: the second row doubles in height
    let work = updated_and_matched_to_full(&page, "3000 rows, tall row", |page| {
        page.toggle_tall();
    });

    // Then: the row's 7 boxes built again, and html and body, which
    // grew; every row after the second shifted
    let rows_after = ROWS - 2;
    assert_eq!(work, (BOXES_PER_ROW + 2, rows_after * BOXES_PER_ROW));
}

#[test]
fn scrolling_the_rows_paints_no_box_and_shifts_every_row() {
    // Given
    let page = rows_page(ROWS);
    page.body_scrolls();

    // When
    let work = updated_and_matched_to_full(&page, "3000 rows, scrolled a row", |page| {
        assert!(page.scroll_body(1.0), "the body scrolls");
    });

    // Then: every row's boxes shifted, none painted
    assert_eq!(work, (0, ROWS * BOXES_PER_ROW));
}

#[test]
fn scrolling_back_shifts_every_row_again() {
    // Given
    let page = rows_page(ROWS);
    page.body_scrolls();
    page.scroll_body(2.0);
    updated(&page);

    // When
    assert!(page.scroll_body(-1.0), "the body scrolls back");
    let work = updated(&page);

    // Then
    assert_eq!(work, (0, ROWS * BOXES_PER_ROW));
}

#[test]
fn a_row_off_whole_device_pixels_is_painted_again_when_it_moves() {
    // Given: html, body, the list and its 4 rows, 25 px each at 1.5x:
    // the second and fourth lie on half device pixels
    let mut list = list("overflow: auto;");
    list.doc
        .add_stylesheet("rows.css", ".row { height: 25px; }")
        .expect("parses");
    let at_1_5 = vapors_elements::Viewport {
        scale: 1.5,
        ..at(1.0)
    };
    list.doc.update(at_1_5).expect("updates");

    // When: a scroll of 2 px, 3 whole device pixels
    assert!(list.doc.scroll_by([10.0, 10.0], [0.0, 2.0]));
    list.doc.update(at_1_5).expect("updates");

    // Then: the first and third rows shifted; shifting the others
    // would not be exact, so they are painted again
    assert_eq!(painted(&list.doc), (2, 2));
}

#[test]
fn a_scroll_by_whole_pixels_shifts_the_rows() {
    // Given
    let mut list = list("overflow: auto;");

    // When: one 40 px row
    assert!(scroll(&mut list.doc, [0.0, 40.0]));
    list.doc.update(at(1.0)).expect("updates");

    // Then
    assert_eq!(painted(&list.doc), (0, 4));
}

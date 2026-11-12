//! Moved slots shift their kept paint in place (IU3 3c-3e,
//! `docs/plans/iu3.md`): at 3000 rows, an edit that resizes a row near
//! the top paints only the boxes it built again and the boxes whose
//! size changed; the rows that only moved are shifted. A scroll moves
//! no box: the rows are in the scroller's frame (IU4, `frames.rs`). A
//! moved box whose shift is not exact (a fractional position) is
//! painted again, never shifted on a guess. A debug build checks every
//! shifted paint against a fresh one inside `update` (`kept_paint.rs`).

mod support;

use support::oracle::assert_matches_full;
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

#[test]
fn resizing_a_row_near_the_top_paints_its_boxes_and_the_body_and_shifts_the_rows_after() {
    // Given
    let page = rows_page(ROWS);
    page.rows_resizable();

    // When: the second row doubles in height
    page.toggle_tall();
    let work = updated(&page);

    // Then: the row's 7 boxes built again, and html and body, which
    // grew; every row after the second shifted
    let rows_after = ROWS - 2;
    assert_eq!(work, (BOXES_PER_ROW + 2, rows_after * BOXES_PER_ROW));
    assert_matches_full(&page.doc, rows_viewport(), "3000 rows, tall row");
}

#[test]
fn scrolling_the_rows_places_the_body_alone_and_moves_paints_and_shifts_no_box() {
    // Given
    let page = rows_page(ROWS);
    page.body_scrolls();

    // When
    assert!(page.scroll_body(1.0), "the body scrolls");
    let work = updated(&page);

    // Then: the body's offset settled, html above it visited, no box
    // moved; the rows are in the body's frame, which the scroll
    // shifted (IU4, `frames.rs`)
    let counts = page.doc.borrow().update_counts();
    assert_eq!((counts.boxes_placed, counts.boxes_moved), (2, 0));
    assert_eq!(work, (0, 0));
    assert_matches_full(&page.doc, rows_viewport(), "3000 rows, scrolled a row");
}

#[test]
fn scrolling_back_paints_and_shifts_no_box() {
    // Given
    let page = rows_page(ROWS);
    page.body_scrolls();
    page.scroll_body(2.0);
    updated(&page);

    // When
    assert!(page.scroll_body(-1.0), "the body scrolls back");
    let work = updated(&page);

    // Then
    assert_eq!(work, (0, 0));
}

#[test]
fn a_scroll_by_whole_device_pixels_paints_no_row_off_whole_device_pixels() {
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

    // Then: the rows stay where they are in the list's frame; the
    // frame's shift is whole device pixels, so none is painted again,
    // and the kept paint check (`kept_paint_check.rs`) holds
    assert_eq!(painted(&list.doc), (0, 0));
}

#[test]
fn a_scroll_by_whole_pixels_paints_and_shifts_no_row() {
    // Given
    let mut list = list("overflow: auto;");

    // When: one 40 px row
    assert!(scroll(&mut list.doc, [0.0, 40.0]));
    list.doc.update(at(1.0)).expect("updates");

    // Then
    assert_eq!(painted(&list.doc), (0, 0));
}

//! The `list` scenario's boxes against the spec (`LIST_ROWS_TOP`): the
//! header across the window, the amount title 24 px from its right edge,
//! row `r` at `y 44 + 52 r`, and only the rows in and near the window.
#![allow(clippy::cast_precision_loss)] // row numbers are small
use super::layout::{Row, assert_rows, at, from_x, row};
use super::list_support::*;
use crate::dashboard::*;

/// The right edge the amount column ends at: 24 px in from the window's.
const AMOUNT_END: f32 = WINDOW.0 - 24.0;
/// Rows touching the 756 px window at offset 0 (14.5 of 52 px), and 4
/// more below it, the `VirtualList`'s default overscan.
const MOUNTED_AT_TOP: usize = 15 + 4;

fn header() -> Vec<Row> {
    let mut rows = vec![row(
        "header".into(),
        "list-head",
        0,
        at(0.0, 0.0, WINDOW.0, TABLE_HEADER_HEIGHT),
    )];
    let lefts = [COL_ORDER, COL_CUSTOMER, COL_STATUS, COL_PROGRESS];
    for (i, left) in lefts.into_iter().enumerate() {
        rows.push(row(
            format!("title {}", COLUMNS[i]),
            "col-title",
            i,
            from_x(left),
        ));
    }
    let sort_x = AMOUNT_END - SORT_WIDTH;
    let sort = at(sort_x, 0.0, SORT_WIDTH, TABLE_HEADER_HEIGHT - 1.0);
    rows.push(row("sort title".into(), "sort", 0, sort));
    rows
}

/// The boxes of display row `r`, the `r`th that exists.
fn list_row(r: usize) -> Vec<Row> {
    let top = LIST_ROWS_TOP + ORDER_ROW_HEIGHT * r as f32;
    let pin = |name: &str, class, want| row(format!("row {r} {name}"), class, r, want);
    let amount = [Some(AMOUNT_END - SORT_WIDTH), None, Some(SORT_WIDTH), None];
    vec![
        pin("row", "order-row", at(0.0, top, WINDOW.0, ORDER_ROW_HEIGHT)),
        pin("number", "order-number", from_x(COL_ORDER)),
        pin("avatar", "order-avatar", from_x(COL_CUSTOMER)),
        pin("pill", "pill", from_x(COL_STATUS)),
        pin("track", "track", from_x(COL_PROGRESS)),
        pin("amount", "c-amount", amount),
    ]
}

#[test]
fn the_header_spans_the_window_with_the_amount_title_24_px_from_its_right_edge() {
    // Given
    let list = list_of(300);

    // Then
    assert_rows(&list.document.borrow(), &header());
    assert_eq!(texts(&list, "col-title"), COLUMNS);
}

#[test]
fn the_rows_that_exist_sit_at_44_plus_52_r_across_the_window() {
    // Given
    let list = list_of(300);

    // Then
    let boxes: Vec<Row> = (0..MOUNTED_AT_TOP).flat_map(list_row).collect();
    assert_rows(&list.document.borrow(), &boxes);
}

#[test]
fn only_the_rows_in_and_near_the_window_exist_and_they_show_the_first_orders() {
    // Given
    let list = list_of(300);

    // Then
    assert_eq!(rows(&list).len(), MOUNTED_AT_TOP);
    assert_eq!(texts(&list, "order-number"), numbers(0..MOUNTED_AT_TOP));
}

#[test]
fn the_row_at_the_top_of_the_window_is_order_0_before_any_scroll() {
    // Given
    let list = list_of(300);

    // Then
    assert_eq!(number_at(&list, LIST_ROWS_TOP + 1.0), "#10200");
    assert_eq!(
        number_at(&list, LIST_ROWS_TOP + ORDER_ROW_HEIGHT + 1.0),
        "#10201"
    );
}

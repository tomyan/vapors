//! The `list` scenario at its full `--rows` (virtual list V3): a million
//! orders, of which only the rows in and near the window exist. The last
//! order is reachable, the count of existing rows is the same as at a
//! thousand, the sort is `list_order`'s, and a list left alone changes
//! nothing.
#![allow(clippy::cast_precision_loss)] // row numbers below 2^24
use super::layout::{Row, assert_rows, at, from_x, row};
use super::list_support::*;
use super::support::by_class;
use crate::dashboard::*;
use crate::list::LIST_HEIGHT;

const MILLION: usize = 1_000_000;

/// Scrolls the list to `y`, clamped, then a frame; answers the offset.
fn scroll_to(list: &crate::list::MountedList, y: f32) -> f32 {
    let container = by_class(&list.document.borrow(), "virtual-list")[0];
    list.document.borrow_mut().scroll_to(container, [0.0, y]);
    frame(list);
    let offset = list.document.borrow().scroll_offset(container);
    offset.expect("a scroll container")[1]
}

fn by_amount(count: usize) -> Vec<usize> {
    list_order(count, true)
        .into_iter()
        .map(|i| i as usize)
        .collect()
}

#[test]
fn the_last_order_is_reachable_at_a_million_rows() {
    // Given
    let list = list_of(MILLION);

    // When: past the end
    let offset = scroll_to(&list, f32::MAX);

    // Then: the range ends a window above the last row's bottom, and the
    // last order fills the bottom of the window
    assert_eq!(offset, ORDER_ROW_HEIGHT * MILLION as f32 - LIST_HEIGHT);
    assert_eq!(number_at(&list, WINDOW.1 - 1.0), "#1010199");
    assert_eq!(
        number_at(&list, WINDOW.1 - ORDER_ROW_HEIGHT - 1.0),
        "#1010198"
    );
}

#[test]
fn the_rows_at_the_end_of_a_million_keep_the_specs_geometry() {
    // Given: at the end, the last row's top is 52 px above the window's bottom
    let list = list_of(MILLION);
    scroll_to(&list, f32::MAX);
    let deepest = rows(&list).len() - 1;
    let top = WINDOW.1 - ORDER_ROW_HEIGHT;
    let mid = |height: f32| top + (ORDER_ROW_HEIGHT - 1.0 - height) / 2.0;

    // Then
    let pin = |name: &str, class, want| row(format!("last row {name}"), class, deepest, want);
    let boxes: Vec<Row> = vec![
        pin("row", "order-row", at(0.0, top, WINDOW.0, ORDER_ROW_HEIGHT)),
        pin(
            "avatar",
            "order-avatar",
            at(COL_CUSTOMER, mid(AVATAR), AVATAR, AVATAR),
        ),
        pin(
            "pill",
            "pill",
            [Some(COL_STATUS), Some(mid(22.0)), None, Some(22.0)],
        ),
        pin(
            "track",
            "track",
            at(
                COL_PROGRESS,
                mid(PROGRESS_HEIGHT),
                PROGRESS_WIDTH,
                PROGRESS_HEIGHT,
            ),
        ),
        pin("number", "order-number", from_x(COL_ORDER)),
    ];
    assert_rows(&list.document.borrow(), &boxes);
}

#[test]
fn as_many_rows_exist_at_a_million_as_at_a_thousand() {
    // Given
    let (small, large) = (list_of(1_000), list_of(MILLION));
    let at_top = (rows(&small).len(), rows(&large).len());

    // When: both scrolled to the middle of a thousand rows
    let middle = ORDER_ROW_HEIGHT * 500.0;
    scroll_to(&small, middle);
    scroll_to(&large, middle);

    // Then: 19 at the top (15 in the window, 4 below), 23 in the middle
    assert_eq!(at_top, (19, 19));
    assert_eq!((rows(&small).len(), rows(&large).len()), (23, 23));
}

#[test]
fn deep_in_a_million_rows_only_the_rows_near_the_window_exist() {
    // Given
    let list = list_of(MILLION);

    // When: row 600,000 at the top
    scroll_to(&list, ORDER_ROW_HEIGHT * 600_000.0);

    // Then
    assert_eq!(texts(&list, "order-number"), numbers(599_996..600_019));
    assert_eq!(number_at(&list, LIST_ROWS_TOP + 1.0), "#610200");
}

#[test]
fn sorting_a_million_rows_shows_list_orders_rows_at_the_top_and_the_end() {
    // Given
    let list = list_of(MILLION);
    let sorted = by_amount(MILLION);

    // When
    click(&list, centre_of(&list, "sort"));

    // Then
    assert_eq!(
        texts(&list, "order-number"),
        numbers(sorted[..19].iter().copied())
    );

    // When
    scroll_to(&list, f32::MAX);

    // Then
    assert_eq!(
        number_at(&list, WINDOW.1 - 1.0),
        numbers([sorted[MILLION - 1]])[0]
    );
}

#[test]
fn a_million_row_list_left_alone_keeps_its_rows_and_reports_no_change() {
    // Given: scrolled deep and drawn
    let list = list_of(MILLION);
    scroll_to(&list, ORDER_ROW_HEIGHT * 777_777.0 + 13.0);
    let before = rows(&list);
    list.document.borrow_mut().take_changed();

    // When
    frame(&list);

    // Then
    assert_eq!(rows(&list), before);
    assert!(!list.document.borrow_mut().take_changed());
}

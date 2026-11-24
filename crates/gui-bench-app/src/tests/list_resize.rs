//! The `list` scenario at a million rows in windows taller and shorter
//! than the spec's (virtual list V4): the list fills the window below the
//! header (`LIST_ROWS_TOP` in the spec), and after a resize shows the
//! orders its new height holds, at the top, deep in the list and at its
//! end.
#![allow(clippy::cast_precision_loss)] // row numbers below 2^24
use super::list_support::*;
use super::support::by_class;
use crate::dashboard::*;

const MILLION: usize = 1_000_000;
/// The two window heights: 200 px taller and 200 px shorter than the
/// spec's 800.
const TALL: f32 = 1000.0;
const SHORT: f32 = 600.0;

/// Scrolls the list to `y`, clamped, then frames in a window `height` px
/// tall; answers the offset.
fn scroll_to(list: &crate::list::MountedList, y: f32, height: f32) -> f32 {
    let container = by_class(&list.document.borrow(), "virtual-list")[0];
    list.document.borrow_mut().scroll_to(container, [0.0, y]);
    settled_at(list, height);
    let offset = list.document.borrow().scroll_offset(container);
    offset.expect("a scroll container")[1]
}

/// The order shown at the top of row `r` of the window, counted from
/// the list's top with no scroll.
fn order(r: usize) -> String {
    numbers([r])[0].clone()
}

#[test]
fn a_taller_window_shows_orders_down_to_its_bottom() {
    // Given
    let list = list_of(MILLION);

    // When
    settled_at(&list, TALL);

    // Then: the window's last pixel row is in row 18 ((999 - 44) / 52)
    assert_eq!(number_at(&list, TALL - 1.0), order(18));
    assert_eq!(number_at(&list, LIST_ROWS_TOP + 1.0), order(0));
}

#[test]
fn a_shorter_window_reaches_the_last_order_at_its_bottom() {
    // Given
    let list = list_of(MILLION);
    settled_at(&list, SHORT);

    // When: past the end
    let offset = scroll_to(&list, f32::MAX, SHORT);

    // Then: the range ends the 556 px list above the last row's bottom
    assert_eq!(
        offset,
        ORDER_ROW_HEIGHT * MILLION as f32 - (SHORT - LIST_ROWS_TOP)
    );
    assert_eq!(number_at(&list, SHORT - 1.0), "#1010199");
    assert_eq!(number_at(&list, SHORT - ORDER_ROW_HEIGHT - 1.0), "#1010198");
}

#[test]
fn deep_in_the_list_a_taller_window_shows_the_orders_below_the_offset() {
    // Given: row 600,000 at the top
    let list = list_of(MILLION);
    scroll_to(&list, ORDER_ROW_HEIGHT * 600_000.0, WINDOW.1);

    // When
    settled_at(&list, TALL);

    // Then
    assert_eq!(number_at(&list, LIST_ROWS_TOP + 1.0), order(600_000));
    assert_eq!(number_at(&list, TALL - 1.0), order(600_018));
}

#[test]
fn at_the_end_a_taller_window_keeps_the_last_order_at_its_bottom() {
    // Given
    let list = list_of(MILLION);
    scroll_to(&list, f32::MAX, WINDOW.1);

    // When: the range shrinks by 200 px, which clamps the offset
    settled_at(&list, TALL);

    // Then
    assert_eq!(number_at(&list, TALL - 1.0), "#1010199");
}

#[test]
fn a_shorter_window_holds_only_the_rows_of_its_height() {
    // Given
    let list = list_of(MILLION);

    // When: 556 px of list show rows 0..10 (11 touch it)
    settled_at(&list, SHORT);

    // Then: those and the 4 of overscan below
    assert_eq!(texts(&list, "order-number"), numbers(0..15));
}

#[test]
fn the_frame_after_a_resize_changes_nothing() {
    // Given: resized and drawn
    let list = list_of(MILLION);
    settled_at(&list, TALL);
    let before = rows(&list);

    // When: the frame the app draws after a change
    frame_at(&list, TALL);

    // Then: no other is asked for, and the rows stay
    assert_eq!(rows(&list), before);
    assert!(!list.document.borrow_mut().take_changed());
}

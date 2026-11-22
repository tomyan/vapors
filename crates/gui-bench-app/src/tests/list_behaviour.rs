//! What the `list` scenario does: a scroll shows the rows at the new
//! offset, the AMOUNT title sorts the rows into `list_order`, and a list
//! left alone changes nothing.
use super::list_support::*;
use crate::dashboard::*;

const COUNT: usize = 300;

fn by_amount(count: usize) -> Vec<usize> {
    list_order(count, true)
        .into_iter()
        .map(|i| i as usize)
        .collect()
}

#[test]
fn a_wheel_scroll_shows_the_rows_at_the_new_offset() {
    // Given
    let list = list_of(COUNT);

    // When: 100 rows and 10 px down
    wheel(&list, ORDER_ROW_HEIGHT * 100.0 + 10.0);

    // Then: order 100 at the top, part of it above the window; rows 100
    // to 114 touch it, and 4 more exist on each side
    assert_eq!(number_at(&list, LIST_ROWS_TOP + 1.0), "#10300");
    assert_eq!(texts(&list, "order-number"), numbers(96..119));
}

#[test]
fn a_click_on_the_amount_title_sorts_the_rows_by_amount_and_a_second_restores_them() {
    // Given
    let list = list_of(COUNT);
    let sort = centre_of(&list, "sort");

    // When
    click(&list, sort);

    // Then
    let sorted = by_amount(COUNT);
    assert_eq!(
        texts(&list, "order-number"),
        numbers(sorted[..19].iter().copied())
    );
    assert_eq!(texts(&list, "sorted"), ["AMOUNT"]);

    // When
    click(&list, sort);

    // Then
    assert_eq!(texts(&list, "order-number"), numbers(0..19));
    assert_eq!(texts(&list, "sorted"), Vec::<String>::new());
}

#[test]
fn sorting_after_a_scroll_keeps_the_offset_and_shows_the_sorted_rows_there() {
    // Given: 50 rows down
    let list = list_of(COUNT);
    wheel(&list, ORDER_ROW_HEIGHT * 50.0);

    // When
    click(&list, centre_of(&list, "sort"));

    // Then
    let sorted = by_amount(COUNT);
    assert_eq!(
        number_at(&list, LIST_ROWS_TOP + 1.0),
        numbers([sorted[50]])[0]
    );
    assert_eq!(
        texts(&list, "order-number"),
        numbers(sorted[46..69].iter().copied())
    );
}

#[test]
fn a_list_left_alone_keeps_its_rows_and_reports_no_change() {
    // Given: scrolled and drawn
    let list = list_of(COUNT);
    wheel(&list, 777.0);
    let before = rows(&list);
    list.document.borrow_mut().take_changed();

    // When
    frame(&list);

    // Then
    assert_eq!(rows(&list), before);
    assert!(!list.document.borrow_mut().take_changed());
}

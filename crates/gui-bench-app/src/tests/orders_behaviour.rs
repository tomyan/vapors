//! What the orders card does: hover, sort, the status filters and the
//! search, through vapors' test drivers. The expected rows are worked out
//! from the spec's own rules (`list_order`, "order 1 is refunded"), not
//! from `visible` and `arranged`, which `dashboard` pins separately.
use super::behaviour_top::{centre_of, texts};
use super::support::*;
use crate::app::{Mounted, order_ids};
use crate::dashboard::*;
use vapors_elements::{Document, TextEdit};

const COUNT: usize = 30;

fn numbers(mounted: &Mounted) -> Vec<String> {
    texts(mounted, "order-number")
}

/// The numbers of orders `indices`, as shown.
fn want(indices: impl IntoIterator<Item = usize>) -> Vec<String> {
    indices
        .into_iter()
        .map(|i| format!("#{}", 10200 + i))
        .collect()
}

fn by_amount(count: usize) -> Vec<usize> {
    list_order(count, true)
        .into_iter()
        .map(|i| i as usize)
        .collect()
}

#[test]
fn the_pointer_over_a_row_gives_it_the_hover_colour_at_once_and_only_it() {
    // Given
    let mounted = with_rows(COUNT);
    let background = |k: usize| paint(&mounted, "order-row", k, "background-color");
    assert_ne!(background(2), css(ROW_HOVER));

    // When
    Document::pointer_moved(&mounted.document, centre_of(&mounted, "order-row", 2));
    refresh(&mounted);

    // Then
    assert_eq!(background(2), css(ROW_HOVER));
    assert_ne!(background(1), css(ROW_HOVER));
    assert_ne!(background(3), css(ROW_HOVER));
}

#[test]
fn a_click_on_the_amount_title_sorts_largest_first_and_the_next_click_restores() {
    // Given
    let mounted = with_rows(COUNT);
    let title = |m: &Mounted| paint(m, "sort", 0, "color");
    assert_eq!(title(&mounted), css(MUTED));

    // When
    click(&mounted, centre_of(&mounted, "sort", 0));

    // Then
    assert_eq!(numbers(&mounted), want(by_amount(COUNT)));
    assert_eq!(title(&mounted), css(PRIMARY));
    click(&mounted, centre_of(&mounted, "sort", 0));
    assert_eq!(numbers(&mounted), want(0..COUNT));
    assert_eq!(title(&mounted), css(MUTED));
}

#[test]
fn a_click_on_another_title_does_not_sort() {
    // Given
    let mounted = with_rows(COUNT);

    // When
    click(&mounted, centre_of(&mounted, "col-title", 0));

    // Then
    assert_eq!(numbers(&mounted), want(0..COUNT));
}

#[test]
fn unchecking_a_status_hides_its_orders_and_the_rows_move_up() {
    // Given
    let mounted = with_rows(COUNT);

    // When: "Refunded" is the fourth checkbox; every fourth order from 1 is refunded
    click(&mounted, centre_of(&mounted, "check", 3));

    // Then
    assert_eq!(numbers(&mounted), want((0..COUNT).filter(|i| i % 4 != 1)));
    let doc = mounted.document.borrow();
    let second = rect(&doc, by_class(&doc, "order-row")[1]);
    assert_eq!(
        second[1],
        TABLE_TOP + TABLE_HEADER_HEIGHT + ORDER_ROW_HEIGHT
    );
}

#[test]
fn typing_in_the_search_input_keeps_the_customers_that_match_without_regard_to_case() {
    // Given: the first 4 characters of a customer, upper-cased
    let mounted = with_rows(COUNT);
    let text: String = order(5)
        .customer
        .chars()
        .take(4)
        .collect::<String>()
        .to_uppercase();
    let length = text.chars().count();

    // When
    click(&mounted, centre_of(&mounted, "search-input", 0));
    let edit = TextEdit {
        before: String::new(),
        text: text.clone(),
        selection: (length, length),
        composing: None,
    };
    Document::text_edited(&mounted.document, &edit).expect("the field takes the edit");
    refresh(&mounted);

    // Then: the customers whose name holds the text, in any case
    let lower = text.to_lowercase();
    let kept = (0..COUNT).filter(|i| order(*i).customer.to_lowercase().contains(&lower));
    let kept: Vec<usize> = kept.collect();
    assert!(!kept.is_empty() && kept.len() < COUNT);
    assert_eq!(numbers(&mounted), want(kept));
}

#[test]
fn the_filters_and_the_sort_combine() {
    // Given
    let mounted = with_rows(COUNT);

    // When
    click(&mounted, centre_of(&mounted, "check", 3));
    click(&mounted, centre_of(&mounted, "sort", 0));

    // Then
    let kept = by_amount(COUNT).into_iter().filter(|i| i % 4 != 1);
    assert_eq!(numbers(&mounted), want(kept));
}

#[test]
#[should_panic(expected = "share the number")]
fn two_orders_with_one_number_are_refused() {
    let base = vec![order(0), order(1), order(0)];
    order_ids(&base, [true; 4], "", false);
}

#[test]
fn the_tables_order_numbers_are_unique() {
    let mut numbers: Vec<u32> = orders(300).iter().map(|o| o.number).collect();
    numbers.sort_unstable();
    numbers.dedup();
    assert_eq!(numbers.len(), 300);
}

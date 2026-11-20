//! What the orders card does: hover, sort, the status filters and the
//! search, through vapors' test drivers.
use super::behaviour_top::{centre_of, texts};
use super::support::*;
use crate::app::Mounted;
use crate::dashboard::*;
use vapors_elements::{Document, TextEdit};

const COUNT: usize = 30;

fn css(color: u32) -> String {
    let [_, r, g, b] = color.to_be_bytes();
    format!("rgb({r} {g} {b})")
}

fn paint(mounted: &Mounted, class: &str, nth: usize, property: &str) -> String {
    let doc = mounted.document.borrow();
    let style = doc
        .computed_style(by_class(&doc, class)[nth])
        .expect("styled");
    style.get_by_name(property).expect("a property").to_css()
}

fn numbers(mounted: &Mounted) -> Vec<String> {
    texts(mounted, "order-number")
}

fn want_numbers(orders: Vec<Order>) -> Vec<String> {
    orders.iter().map(Order::number_text).collect()
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
    assert_eq!(
        numbers(&mounted),
        want_numbers(arranged(orders(COUNT), true))
    );
    assert_eq!(title(&mounted), css(PRIMARY));
    click(&mounted, centre_of(&mounted, "sort", 0));
    assert_eq!(numbers(&mounted), want_numbers(orders(COUNT)));
    assert_eq!(title(&mounted), css(MUTED));
}

#[test]
fn a_click_on_another_title_does_not_sort() {
    // Given
    let mounted = with_rows(COUNT);

    // When
    click(&mounted, centre_of(&mounted, "col-title", 0));

    // Then
    assert_eq!(numbers(&mounted), want_numbers(orders(COUNT)));
}

#[test]
fn unchecking_a_status_hides_its_orders_and_the_rows_move_up() {
    // Given
    let mounted = with_rows(COUNT);

    // When: "Refunded" is the fourth checkbox
    click(&mounted, centre_of(&mounted, "check", 3));

    // Then
    let kept: Vec<Order> = orders(COUNT)
        .into_iter()
        .filter(|o| o.status != Status::Refunded)
        .collect();
    assert!(kept.len() < COUNT);
    assert_eq!(numbers(&mounted), want_numbers(kept));
    let doc = mounted.document.borrow();
    let second = rect(&doc, by_class(&doc, "order-row")[1]);
    assert_eq!(
        second[1],
        TABLE_TOP + TABLE_HEADER_HEIGHT + ORDER_ROW_HEIGHT
    );
}

#[test]
fn typing_in_the_search_input_keeps_the_customers_that_match_without_regard_to_case() {
    // Given
    let mounted = with_rows(COUNT);
    let text = order(5).customer.to_uppercase()[..4].to_string();

    // When
    click(&mounted, centre_of(&mounted, "search-input", 0));
    let edit = TextEdit {
        before: String::new(),
        text: text.clone(),
        selection: (4, 4),
        composing: None,
    };
    Document::text_edited(&mounted.document, &edit).expect("the field takes the edit");
    refresh(&mounted);

    // Then
    let kept: Vec<Order> = orders(COUNT)
        .into_iter()
        .filter(|o| visible(o, &[true; 4], &text))
        .collect();
    assert!(!kept.is_empty() && kept.len() < COUNT);
    assert_eq!(numbers(&mounted), want_numbers(kept));
}

#[test]
fn the_filters_and_the_sort_combine() {
    // Given
    let mounted = with_rows(COUNT);

    // When
    click(&mounted, centre_of(&mounted, "check", 3));
    click(&mounted, centre_of(&mounted, "sort", 0));

    // Then
    let kept: Vec<Order> = orders(COUNT)
        .into_iter()
        .filter(|o| visible(o, &[true, true, true, false], ""))
        .collect();
    assert_eq!(numbers(&mounted), want_numbers(arranged(kept, true)));
}

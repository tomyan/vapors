//! The spec's rectangles for the orders card: the header, and a sample of
//! rows at 30 and 300 rows.
#![allow(clippy::cast_precision_loss)] // indexes and counts are small
use super::layout::{Row, assert_rows, at, row};
use super::support::*;
use crate::dashboard::*;

const CARD_X: f32 = SIDEBAR_WIDTH + CONTENT_PADDING;
const CARD_WIDTH: f32 = WINDOW.0 - SIDEBAR_WIDTH - 2.0 * CONTENT_PADDING;
/// The row's content height: its 1 px rule is inside the 52.
const BODY: f32 = ORDER_ROW_HEIGHT - 1.0;

fn header() -> Vec<Row> {
    let head = at(CARD_X, TABLE_TOP, CARD_WIDTH, TABLE_HEADER_HEIGHT);
    let mut rows = vec![row("header".into(), "orders-head", 0, head)];
    let lefts = [COL_ORDER, COL_CUSTOMER, COL_STATUS, COL_PROGRESS, SORT_X];
    for (i, left) in lefts.into_iter().enumerate() {
        let want = [Some(CARD_X + left), Some(TABLE_TOP), None, None];
        rows.push(row(format!("title {}", COLUMNS[i]), "col-title", i, want));
    }
    // The title's height is the header's less its 1 px rule.
    let height = TABLE_HEADER_HEIGHT - 1.0;
    let sort = at(CARD_X + SORT_X, TABLE_TOP, SORT_WIDTH, height);
    rows.push(row("sort title".into(), "sort", 0, sort));
    rows
}

/// The boxes of display row `row_no`, which shows order `index`.
fn order_row(row_no: usize, index: usize) -> Vec<Row> {
    let found = order(index);
    let top = TABLE_TOP + TABLE_HEADER_HEIGHT + ORDER_ROW_HEIGHT * row_no as f32;
    let mid = |height: f32| top + (BODY - height) / 2.0;
    let col = |left: f32| CARD_X + left;
    let pin = |name: &str, class, want| row(format!("row {row_no} {name}"), class, row_no, want);
    let fill = found.progress as f32 * PROGRESS_WIDTH / 100.0;
    vec![
        pin(
            "row",
            "order-row",
            at(CARD_X, top, CARD_WIDTH, ORDER_ROW_HEIGHT),
        ),
        pin(
            "avatar",
            "order-avatar",
            at(col(COL_CUSTOMER), mid(AVATAR), AVATAR, AVATAR),
        ),
        pin(
            "name",
            "order-name",
            [Some(col(COL_CUSTOMER + 40.0)), None, None, None],
        ),
        pin(
            "pill",
            "pill",
            [Some(col(COL_STATUS)), Some(mid(22.0)), None, Some(22.0)],
        ),
        pin(
            "track",
            "track",
            at(col(COL_PROGRESS), mid(6.0), PROGRESS_WIDTH, PROGRESS_HEIGHT),
        ),
        pin(
            "fill",
            "fill",
            [Some(col(COL_PROGRESS)), None, Some(fill), Some(6.0)],
        ),
        pin(
            "number",
            "order-number",
            [Some(col(COL_ORDER)), None, None, None],
        ),
    ]
}

fn sample(count: usize) -> Vec<Row> {
    assert!(count > 3, "the sample needs rows 0 to 3");
    let mut rows = header();
    for r in [0, 1, 2, 3, 17, count / 2, count - 1] {
        rows.extend(order_row(r, r));
        rows.push(amount(r));
    }
    rows
}

/// The amount's cell is the sort title's column, which ends `COL_AMOUNT_END`
/// from the card's left.
fn amount(r: usize) -> Row {
    assert_eq!(SORT_X + SORT_WIDTH, COL_AMOUNT_END);
    let want = [Some(CARD_X + SORT_X), None, Some(SORT_WIDTH), None];
    row(format!("row {r} amount"), "c-amount", r, want)
}

#[test]
fn the_header_and_a_sample_of_rows_are_where_the_spec_puts_them_at_30_and_300_rows() {
    for count in [30, 300] {
        // Given
        let mounted = with_rows(count);
        let doc = mounted.document.borrow();

        // Then
        assert_eq!(by_class(&doc, "order-row").len(), count);
        assert_rows(&doc, &sample(count));
    }
}

#[test]
fn the_rows_show_the_orders_texts() {
    // Given
    let mounted = with_rows(30);
    let doc = mounted.document.borrow();
    let texts = |class: &str| -> Vec<String> {
        let nodes = by_class(&doc, class);
        nodes.into_iter().map(|node| text(&doc, node)).collect()
    };
    let orders = orders(30);

    // Then
    assert_eq!(texts("col-title"), COLUMNS);
    let want = |f: fn(&Order) -> String| orders.iter().map(f).collect::<Vec<_>>();
    assert_eq!(texts("order-number"), want(Order::number_text));
    assert_eq!(texts("order-name"), want(|o| o.customer.clone()));
    assert_eq!(texts("order-amount"), want(Order::amount));
    assert_eq!(texts("pill"), want(|o| o.status.label().into()));
    assert_eq!(texts("order-avatar"), want(|o| o.initials.clone()));
}

#[test]
fn every_pill_avatar_and_bar_has_the_colours_of_its_order() {
    // Given
    let mounted = with_rows(30);

    // Then
    for i in 0..30 {
        let o = order(i);
        let (back, fore) = o.status.colors();
        let paint = |class, property| paint(&mounted, class, i, property);
        assert_eq!(paint("pill", "background-color"), css(back), "pill {i}");
        assert_eq!(paint("pill", "color"), css(fore), "pill text {i}");
        assert_eq!(
            paint("order-avatar", "background-color"),
            css(o.avatar),
            "avatar {i}"
        );
        assert_eq!(paint("fill", "background-color"), css(PRIMARY));
        assert_eq!(paint("track", "background-color"), css(TRACK));
        assert_eq!(paint("order-row", "border-bottom-color"), css(ROW_RULE));
    }
}

#[test]
fn every_fill_is_its_percentage_of_the_track() {
    // Given: the first 101 orders
    let mounted = with_rows(101);
    let doc = mounted.document.borrow();

    // Then: they carry every percentage, 0..=100, and each fill is its share
    let mut seen: Vec<u32> = (0..101).map(|i| order(i).progress).collect();
    seen.sort_unstable();
    assert_eq!(seen, (0..=100).collect::<Vec<u32>>());
    for i in 0..101 {
        let [_, _, w, _] = rect(&doc, by_class(&doc, "fill")[i]);
        let want = order(i).progress as f32 * PROGRESS_WIDTH / 100.0;
        assert!((w - want).abs() < 0.01, "order {i}: {w} for {want}");
    }
}

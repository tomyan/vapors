//! The columns the mirror declines, each named, with the rest of the
//! page still answered as Taffy did.

use super::compare::compare;
use crate::lane_decline::LaneDecline;
use crate::layout::lane_page::{element, page};

/// The declines of a page of one column styled `.col { css }` holding
/// two items, the second styled `.item { item }`.
fn declines(css: &str, item: &str) -> Vec<LaneDecline> {
    let css = format!(
        "body {{ margin: 0; }} .col {{ display: flex; flex-direction: column; {css} }} \
         .item {{ {item} }}"
    );
    let mut doc = page(&css, |doc, body| {
        let col = element(doc, body, "col", None);
        element(doc, col, "plain", Some("plain"));
        element(doc, col, "item", Some("item"));
    });
    compare(&mut doc).declined
}

#[test]
fn each_column_case_a_lane_does_not_lay_out_is_declined_by_name() {
    let cases = [
        (
            "flex-direction: column-reverse",
            "",
            LaneDecline::ColumnReverse,
        ),
        ("flex-direction: row", "", LaneDecline::FlexRow),
        ("flex-wrap: wrap", "", LaneDecline::FlexWrap),
        ("justify-content: center", "", LaneDecline::JustifyContent),
        (
            "justify-content: space-between",
            "",
            LaneDecline::JustifyContent,
        ),
        ("row-gap: 10%", "", LaneDecline::RelativeGap),
        ("", "flex-grow: 1", LaneDecline::FlexGrow),
        ("", "margin-top: auto", LaneDecline::MainAutoMargin),
        ("", "position: absolute", LaneDecline::OutOfFlowChild),
    ];
    for (css, item, decline) in cases {
        // When
        let declined = declines(css, item);

        // Then: each ask of the column is declined for that reason
        assert!(!declined.is_empty(), "{css} / {item}: nothing declined");
        assert!(
            declined.iter().all(|&named| named == decline),
            "{css} / {item}: {declined:?}, expected {decline:?}"
        );
    }
}

#[test]
fn a_column_shorter_than_items_that_can_shrink_is_declined() {
    // Given: a column of definite height whose second item clips its
    // content, so its minimum height is 0 and it shrinks
    let declined = declines("height: 20px;", "height: 30px; overflow: hidden;");

    // Then
    assert_eq!(declined, [LaneDecline::Shrinks]);
}

#[test]
fn a_scrolling_column_with_an_item_wholly_before_its_origin_is_declined() {
    let declined = declines(
        "height: 40px; overflow: auto;",
        "flex-shrink: 0; height: 10px; margin-top: -60px;",
    );
    assert_eq!(declined, [LaneDecline::BeforeScrollOrigin]);
}

#[test]
fn an_auto_height_column_whose_items_outgrow_their_bases_and_can_shrink_is_declined() {
    // Given: a column sized by its parent from its items' bases (a
    // non-stretched item of a flex row of definite height); one item's
    // basis is below its content, so the items outgrow the column, and
    // another clips its content, so it can shrink
    let css = "body { margin: 0; } .row { display: flex; height: 100px; align-items: flex-start; } \
               .col { display: flex; flex-direction: column; width: 80px; } \
               .small { flex-basis: 4px; } .clip { height: 30px; overflow: hidden; }";
    let mut doc = page(css, |doc, body| {
        let row = element(doc, body, "row", None);
        let col = element(doc, row, "col", None);
        element(doc, col, "small", Some("small"));
        element(doc, col, "clip", Some("clip"));
    });

    // When
    let declined = compare(&mut doc).declined;

    // Then: the row; the column's ask with no width; its layout, at
    // the height of the bases, shrinks the clipping item
    assert_eq!(
        declined,
        [
            LaneDecline::FlexRow,
            LaneDecline::WidthUnknown,
            LaneDecline::Shrinks
        ]
    );
}

#[test]
fn an_item_that_can_shrink_with_a_floor_above_its_maximum_is_declined() {
    // Given: a column taller than its items, one of which takes a floor
    // of 100px from its aspect ratio and width but has `max-height:
    // 50px`, which Taffy clamps it to (the review's page)
    let declined = declines(
        "width: 300px; height: 500px;",
        "aspect-ratio: 1; width: 100px; min-width: 100px; max-height: 50px; align-self: flex-start;",
    );

    // Then
    assert_eq!(declined, [LaneDecline::ClampedHeight]);
}

#[test]
fn a_column_with_no_items_is_declined() {
    // Given: an empty column, which vapors lays out as a leaf
    let css = "body { margin: 0; } .col { display: flex; flex-direction: column; width: 300px; \
               height: 100px; padding: 10px; }";
    let mut doc = page(css, |doc, body| {
        element(doc, body, "col", None);
    });

    // When
    let declined = compare(&mut doc).declined;

    // Then
    assert_eq!(declined, [LaneDecline::NoItems]);
}

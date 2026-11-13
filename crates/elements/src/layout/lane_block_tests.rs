//! The mirror answers as Taffy does on pages of each CSS case a block
//! lane lays out (`lane_block_compare.rs` is the comparison). Every
//! size here is a multiple of a quarter, so Taffy's f32 running sums
//! and the fold's f64 sums agree to the bit.

use compare::{compare, element, page};

use crate::lane_decline::LaneDecline;

#[path = "lane_block_compare.rs"]
mod compare;

#[test]
fn rows_of_flex_children_with_margins_are_laid_out_as_taffy_lays_them() {
    // Given: the rows page's shape, with margins collapsing between rows
    let css = "body { margin: 0; } .list { padding: 6px 0; } \
               .row { display: flex; height: 20px; margin: 4px 0; } .cell { width: 40px; }";
    let mut doc = page(css, |doc, body| {
        let list = element(doc, body, "list", None);
        for index in 0..5 {
            let row = element(doc, list, "row", None);
            element(doc, row, "cell", Some(&format!("row {index}")));
        }
    });

    // When
    let compared = compare(&mut doc);

    // Then: html, body and the list; the rows are flex containers, and
    // each cell is asked by its row for its size with no width known
    assert_eq!(compared.containers, 3, "{compared:?}");
    assert!(compared.asks >= 3, "{compared:?}");
    assert_eq!(compared.declined, [LaneDecline::WidthUnknown; 5]);
}

#[test]
fn block_children_with_text_collapse_margins_and_carry_baselines() {
    // Given: blocks of text with positive and negative margins, one
    // whose first child's margin collapses through to its top
    let css = "body { margin: 8px; } .a { margin: 12px 0 -4px; } .b { margin: 6px 0 20px; } \
               .outer { margin-top: 2px; } .inner { margin-top: 10px; } \
               .padded { padding: 3px 5px; border: 1px solid black; margin: 2px; }";
    let mut doc = page(css, |doc, body| {
        element(doc, body, "a", Some("first"));
        element(doc, body, "b", Some("second"));
        let outer = element(doc, body, "outer", None);
        element(doc, outer, "inner", Some("inner"));
        element(doc, body, "padded", Some("padded"));
    });

    // When
    let compared = compare(&mut doc);

    // Then: html, body, a, b, outer, inner and padded
    assert_eq!(compared.containers, 7, "{compared:?}");
    assert_eq!(compared.declined, []);
}

#[test]
fn container_sizes_percentages_and_relative_children_match() {
    // Given
    let css = "body { margin: 0; } \
               .box { width: 200px; height: 100px; min-height: 50px; max-height: 150px; \
                      padding: 4px; box-sizing: content-box; } \
               .half { width: 50%; margin: 0 10%; padding: 5%; } \
               .rel { position: relative; top: 3px; left: -2px; } \
               .centred { width: 100px; margin: 0 auto; } \
               .capped { max-height: 10px; min-height: 2px; }";
    let mut doc = page(css, |doc, body| {
        let container = element(doc, body, "box", None);
        element(doc, container, "half", Some("half"));
        element(doc, container, "rel", Some("relative"));
        element(doc, container, "centred", Some("centred"));
        element(doc, container, "capped", Some("capped"));
    });

    // When
    let compared = compare(&mut doc);

    // Then
    assert_eq!(compared.containers, 7, "{compared:?}");
    assert_eq!(compared.declined, []);
}

#[test]
fn a_scroll_container_answers_its_overflow_and_gutter() {
    // Given: a list shorter than its rows, scrolling
    let css = "body { margin: 0; } .list { height: 40px; overflow: auto; padding: 2px; } \
               .scroll { height: 40px; overflow: scroll; } .row { height: 20px; }";
    let mut doc = page(css, |doc, body| {
        let list = element(doc, body, "list", None);
        for _ in 0..4 {
            element(doc, list, "row", Some("row"));
        }
        let scroll = element(doc, body, "scroll", None);
        for _ in 0..4 {
            element(doc, scroll, "row", Some("row"));
        }
    });

    // When
    let compared = compare(&mut doc);

    // Then: html, body, two lists, eight rows
    assert_eq!(compared.containers, 12, "{compared:?}");
    assert_eq!(compared.declined, []);
}

#[test]
fn an_empty_child_declines_its_container_as_collapsing_through() {
    // Given: an empty block among the rows
    let css = "body { margin: 0; } .row { height: 20px; } .empty { margin: 5px 0; }";
    let mut doc = page(css, |doc, body| {
        element(doc, body, "row", Some("row"));
        element(doc, body, "empty", None);
        element(doc, body, "row", Some("row"));
    });

    // When
    let compared = compare(&mut doc);

    // Then: body is declined; html and the rows compare
    assert_eq!(compared.declined, [LaneDecline::CollapsesThrough]);
    assert_eq!(compared.containers, 3, "{compared:?}");
}

#[test]
fn an_absolute_child_declines_its_container() {
    let css = "body { margin: 0; position: relative; } .row { height: 20px; } \
               .abs { position: absolute; top: 0; left: 0; }";
    let mut doc = page(css, |doc, body| {
        element(doc, body, "row", Some("row"));
        element(doc, body, "abs", Some("floating"));
    });
    let compared = compare(&mut doc);

    // Then: body for the child; the child itself is asked with no
    // width known when it is placed
    assert_eq!(
        compared.declined,
        [LaneDecline::OutOfFlowChild, LaneDecline::WidthUnknown]
    );
}

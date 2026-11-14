//! The mirror answers as Taffy does on pages of each CSS case a block
//! lane lays out (`lane_block_compare.rs` is the comparison, and
//! accounts for every ask). Every size here is a multiple of a
//! quarter, so Taffy's f32 running sums and the fold's f64 sums agree
//! to the bit.

use compare::compare;

use crate::layout::lane_compare::Compared;
use crate::layout::lane_page::{element, page};

use crate::lane_decline::LaneDecline;

#[path = "lane_block_compare.rs"]
mod compare;
#[path = "lane_block_decline_tests.rs"]
mod declines;

/// The page's block-in-block containers were all answered as Taffy
/// did, each asked once, for its layout.
fn assert_all_laid_out(compared: &Compared, containers: usize) {
    let expected = Compared {
        containers,
        layout_asks: containers,
        ..Compared::default()
    };
    assert_eq!(*compared, expected);
}

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

    // Then: html, body and the list are laid out; each cell, asked by
    // its flex row for its content size with no width known, declines
    // that ask, and is laid out with its width known
    let expected = Compared {
        containers: 3,
        short: 0,
        size_asks: 0,
        layout_asks: 8,
        unanswered: 0,
        declined: vec![LaneDecline::WidthUnknown; 5],
    };
    assert_eq!(compared, expected);
}

#[test]
fn block_children_with_text_collapse_margins_and_carry_baselines() {
    // Given: blocks of text with positive and negative margins, one
    // whose first child's margin collapses through to its top, and a
    // first child with no baseline before one with: Taffy's leaves have
    // none, so the second is a scroll container, whose baseline Taffy
    // makes from its height
    let css = "body { margin: 8px; } .blank { height: 12px; } \
               .scroller { height: 8px; overflow: auto; } \
               .a { margin: 12px 0 -4px; } .b { margin: 6px 0 20px; } \
               .outer { margin-top: 2px; } .inner { margin-top: 10px; } \
               .padded { padding: 3px 5px; border: 1px solid black; margin: 2px; }";
    let mut doc = page(css, |doc, body| {
        element(doc, body, "blank", None);
        element(doc, body, "scroller", Some("scrolls"));
        element(doc, body, "a", Some("first"));
        element(doc, body, "b", Some("second"));
        let outer = element(doc, body, "outer", None);
        element(doc, outer, "inner", Some("inner"));
        element(doc, body, "padded", Some("padded"));
    });

    // When
    let compared = compare(&mut doc);

    // Then: html, body, scroller, a, b, outer, inner and padded
    assert_all_laid_out(&compared, 8);
}

#[test]
fn container_sizes_percentages_and_relative_children_match() {
    // Given: a definite-height container with percentage sizes and
    // relative insets inside, an auto-height one with a percentage
    // height (indefinite), and min-height above the content
    let css = "body { margin: 0; } \
               .box { width: 200px; height: 100px; min-height: 50px; max-height: 150px; \
                      padding: 4px; box-sizing: content-box; } \
               .half { width: 50%; height: 50%; margin: 0 10%; padding: 5%; } \
               .rel { position: relative; top: 3px; left: -2px; } \
               .rel2 { position: relative; bottom: 4px; right: 10%; } \
               .centred { width: 100px; margin: 0 auto; } \
               .capped { max-height: 10px; min-height: 2px; } \
               .auto { padding: 2px; } .pct { height: 50%; } \
               .tall { min-height: 60px; } .kid { height: 10px; margin-bottom: 8px; }";
    let mut doc = page(css, |doc, body| {
        let container = element(doc, body, "box", None);
        element(doc, container, "half", Some("half"));
        element(doc, container, "rel", Some("relative"));
        element(doc, container, "rel2", Some("relative too"));
        element(doc, container, "centred", Some("centred"));
        element(doc, container, "capped", Some("capped"));
        let auto = element(doc, body, "auto", None);
        element(doc, auto, "pct", Some("indefinite"));
        let tall = element(doc, body, "tall", None);
        element(doc, tall, "kid", None);
    });

    // When
    let compared = compare(&mut doc);

    // Then: html, body, box, its five children, auto, pct, tall
    assert_all_laid_out(&compared, 11);
}

#[test]
fn a_scroll_container_answers_its_overflow_and_gutter() {
    // Given: lists shorter than their rows, scrolling, one whose first
    // row starts above the origin but ends below it
    let css = "body { margin: 0; } .list { height: 40px; overflow: auto; padding: 2px; } \
               .scroll { height: 40px; overflow: scroll; } .row { height: 20px; } \
               .up { margin-top: -10px; }";
    let mut doc = page(css, |doc, body| {
        let list = element(doc, body, "list", None);
        element(doc, list, "row up", Some("row"));
        for _ in 0..3 {
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
    assert_all_laid_out(&compared, 12);
}

#[test]
fn flex_items_ask_blocks_size_queries_and_short_answers() {
    // Given: a definite-width block in a flex column, which the column
    // asks for its size, and a block whose aspect ratio gives its
    // height from its width, which a flex row asks with both known
    let css = "body { margin: 0; } .column { display: flex; flex-direction: column; } \
               .row { display: flex; } .sized { width: 40px; } \
               .ratio { width: 40px; aspect-ratio: 2; }";
    let mut doc = page(css, |doc, body| {
        let column = element(doc, body, "column", None);
        element(doc, column, "sized", Some("sized"));
        let row = element(doc, body, "row", None);
        element(doc, row, "ratio", Some("ratio"));
    });

    // When
    let compared = compare(&mut doc);

    // Then: html, body, sized and ratio; sized answers two size
    // queries, ratio one short answer, each one layout
    let expected = Compared {
        containers: 4,
        short: 1,
        size_asks: 2,
        layout_asks: 4,
        unanswered: 0,
        declined: vec![],
    };
    assert_eq!(compared, expected);
}

#[test]
fn a_scroll_container_with_an_empty_child_is_laid_out() {
    // Given: a scrolling list holding a child of no width, which adds
    // nothing to the list's overflow
    let css = "body { margin: 0; } .list { height: 40px; overflow: auto; } \
               .row { height: 20px; } .empty { width: 0; height: 10px; }";
    let mut doc = page(css, |doc, body| {
        let list = element(doc, body, "list", None);
        element(doc, list, "row", Some("row"));
        element(doc, list, "empty", None);
        element(doc, list, "row", Some("row"));
    });

    // When
    let compared = compare(&mut doc);

    // Then: html, body, the list and its two rows
    assert_all_laid_out(&compared, 5);
}

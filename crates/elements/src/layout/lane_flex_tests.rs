//! The column mirror answers as Taffy does on pages of each CSS case a
//! flex lane lays out (`docs/plans/splice-layout.md`, section 10.3;
//! `lane_flex_compare.rs` is the comparison). Every size here is a
//! multiple of a quarter, so Taffy's f32 running sums and the fold's
//! f64 sums agree to the bit.

use compare::compare;

use crate::lane_decline::LaneDecline;
use crate::layout::lane_compare::Compared;
use crate::layout::lane_page::{element, page};

#[path = "lane_flex_compare.rs"]
mod compare;
#[path = "lane_flex_decline_tests.rs"]
mod declines;

/// The page's columns were all answered as Taffy did, each laid out
/// once, none asked otherwise.
fn assert_all_laid_out(compared: &Compared, containers: usize) {
    let expected = Compared {
        containers,
        layout_asks: containers,
        ..Compared::default()
    };
    assert_eq!(*compared, expected);
}

#[test]
fn a_column_of_flex_rows_is_laid_out_as_taffy_lays_it() {
    // Given: the rows page's shape as a flex column, rows of cells
    let css = "body { margin: 0; } .list { display: flex; flex-direction: column; } \
               .row { display: flex; height: 20px; margin: 2px 0 3px; } .cell { width: 40px; }";
    let mut doc = page(css, |doc, body| {
        let list = element(doc, body, "list", None);
        for _ in 0..4 {
            let row = element(doc, list, "row", None);
            element(doc, row, "cell", Some("cell"));
        }
    });

    // When
    let compared = compare(&mut doc);

    // Then: the list is laid out; each row, a flex row, is declined
    // each of the three ways the list asks it (two size queries and a
    // layout)
    let expected = Compared {
        containers: 1,
        layout_asks: 1,
        declined: vec![LaneDecline::FlexRow; 12],
        ..Compared::default()
    };
    assert_eq!(compared, expected);
}

#[test]
fn items_with_gaps_edges_and_limits_match() {
    // Given: a column with a gap, padding and border, its items with
    // margins, padding, limits, a basis above their content and an
    // aspect ratio; and two whose share of the column's height is not
    // their basis: one with a basis below its content, one with a basis
    // above its maximum
    let css = "body { margin: 0; } \
               .col { display: flex; flex-direction: column; row-gap: 6px; padding: 3px 5px; \
                      border: 1px solid black; } \
               .a { margin: 4px 8px 2px; padding: 2px; } .b { flex-basis: 30px; } \
               .c { min-height: 25px; } .d { max-height: 8px; } \
               .e { width: 40px; aspect-ratio: 2; } .f { height: 12px; flex-shrink: 0; } \
               .g { flex-basis: 10px; } .i { flex-basis: 30px; max-height: 20px; }";
    let mut doc = page(css, |doc, body| {
        let col = element(doc, body, "col", None);
        for class in ["a", "b", "c", "d", "e", "f", "g", "i"] {
            element(doc, col, class, Some(class));
        }
    });

    // When
    let compared = compare(&mut doc);

    // Then
    assert_all_laid_out(&compared, 1);
}

#[test]
fn items_aligned_across_and_offset_match() {
    // Given: items aligned each way across, by auto margins, and moved
    // by relative insets
    let css = "body { margin: 0; } .col { display: flex; flex-direction: column; \
               align-items: center; width: 200px; } \
               .start { align-self: flex-start; } .end { align-self: flex-end; } \
               .wide { align-self: stretch; } .base { align-self: baseline; } \
               .auto { margin: 0 auto; width: 50px; } .left { margin-left: auto; width: 60px; } \
               .rel { position: relative; top: 3px; left: -2px; } \
               .rel2 { position: relative; bottom: 4px; right: 10%; }";
    let mut doc = page(css, |doc, body| {
        let col = element(doc, body, "col", None);
        for class in [
            "start", "end", "wide", "base", "auto", "left", "rel", "rel2", "plain",
        ] {
            element(doc, col, class, Some(class));
        }
    });

    // When
    let compared = compare(&mut doc);

    // Then
    assert_all_laid_out(&compared, 1);
}

#[test]
fn a_definite_height_column_whose_items_do_not_shrink_matches() {
    // Given: columns shorter and taller than their items, whose items
    // do not shrink, or are no taller than their content, and a
    // scrolling column with an item starting above its origin
    let css = "body { margin: 0; } .col { display: flex; flex-direction: column; } \
               .short { height: 30px; } .tall { height: 300px; } \
               .scroll { height: 40px; overflow: auto; padding: 2px; } \
               .fixed { height: 20px; flex-shrink: 0; } .text { margin-bottom: 4px; } \
               .up { margin-top: -10px; }";
    let mut doc = page(css, |doc, body| {
        for class in ["col short", "col tall", "col scroll"] {
            let col = element(doc, body, class, None);
            let first = if class == "col scroll" {
                "fixed up"
            } else {
                "fixed"
            };
            element(doc, col, first, Some("fixed"));
            element(doc, col, "text", Some("text"));
            element(doc, col, "fixed", Some("fixed"));
        }
    });

    // When
    let compared = compare(&mut doc);

    // Then
    assert_all_laid_out(&compared, 3);
}

#[test]
fn a_column_in_a_flex_row_answers_its_size_queries() {
    // Given: a column of known width as an item of a flex row, which
    // asks it for its height
    let css = "body { margin: 0; } .row { display: flex; } \
               .col { display: flex; flex-direction: column; width: 80px; } \
               .item { height: 10px; }";
    let mut doc = page(css, |doc, body| {
        let row = element(doc, body, "row", None);
        let col = element(doc, row, "col", None);
        element(doc, col, "item", None);
        element(doc, col, "item", Some("item"));
    });

    // When
    let compared = compare(&mut doc);

    // Then: the row is declined; the column declines the row's ask for
    // its content size, with no width known, as a block cell does, and
    // answers the size query and the layout with its width known
    let expected = Compared {
        size_asks: 1,
        layout_asks: 1,
        declined: vec![LaneDecline::FlexRow, LaneDecline::WidthUnknown],
        ..Compared::default()
    };
    assert_eq!(compared, expected);
}

#[test]
fn a_column_measured_in_a_definite_height_sums_its_items_bases() {
    // Given: a column of known width, not stretched, in a flex row of
    // definite height, which measures it with that height available
    // and its own unknown; its items' bases and minimums differ from
    // their content
    let css = "body { margin: 0; } .row { display: flex; height: 100px; align-items: flex-start; } \
               .col { display: flex; flex-direction: column; width: 80px; } \
               .based { flex-basis: 30px; } .least { min-height: 22px; margin: 1px 0 3px; }";
    let mut doc = page(css, |doc, body| {
        let row = element(doc, body, "row", None);
        let col = element(doc, row, "col", None);
        element(doc, col, "based", Some("based"));
        element(doc, col, "least", Some("least"));
        element(doc, col, "plain", Some("plain"));
    });

    // When
    let compared = compare(&mut doc);

    // Then: the row is declined, and the column its ask for its
    // content size with no width known; the column answers the size
    // query, its width known and the row's height available, and the
    // layout
    let expected = Compared {
        size_asks: 1,
        layout_asks: 1,
        declined: vec![LaneDecline::FlexRow, LaneDecline::WidthUnknown],
        ..Compared::default()
    };
    assert_eq!(compared, expected);
}

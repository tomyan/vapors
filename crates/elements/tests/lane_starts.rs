//! A lane child's layout holds its relative offset along the axis, not
//! its start (`docs/plans/splice-layout.md`, section 11.2, 4c): after an
//! insert in a framed list layout writes no later row and placement
//! visits none; a lane that is not framed is placed with its starts
//! added back, and a scroll container's range counts its lane's starts.

use crate::support;

use support::narrow_list::{List, ROWS, VIEW, list};
use support::oracle::assert_matches_full;
use vapors_dom::Dom;
use vapors_elements::{LaneDecline, UpdateCounts};

/// The counts of the update after `edit` on a list of `rows`, styled
/// `css` besides the rows, which matches a full update.
fn after(rows: usize, css: &str, edit: impl Fn(&List)) -> UpdateCounts {
    let list = list(rows, &format!("{ROWS} {css}"));
    edit(&list);
    let counts = list.update();
    assert_matches_full(&list.doc, VIEW(), "after the edit");
    counts
}

#[test]
fn an_insert_places_no_later_row() {
    // When: a row inserted in the middle of 100 rows and of 500
    let small = after(100, "", |list| _ = list.insert_at(50));
    let large = after(500, "", |list| _ = list.insert_at(250));

    // Then: the new row's boxes and their ancestors alone, at either size
    assert_eq!(
        small.boxes_placed, large.boxes_placed,
        "placement visits later rows: {small:?} {large:?}"
    );
}

#[test]
fn an_insert_in_a_list_that_is_not_framed_places_later_rows_where_they_start() {
    // Given: a row a fraction of a pixel taller, so the list is not framed
    let css = ".odd { height: 20.5px; }";

    // When: a row inserted before it, after the first update
    let counts = after(100, css, |list| {
        list.doc
            .borrow_mut()
            .set_attribute(list.rows[60], "class", "row odd");
        list.update();
        list.insert_at(30);
    });

    // Then: placed as a full update places it (`after`), and counted
    assert_eq!(
        counts.lane_unframed,
        Some(LaneDecline::LaneFramePixel),
        "{counts:?}"
    );
}

#[test]
fn an_insert_in_a_short_list_places_later_rows_where_they_start() {
    // When: fewer rows than a lane frames, a row inserted before others
    let counts = after(10, "", |list| _ = list.insert_at(2));

    // Then: placed as a full update places it (`after`), not counted
    assert_eq!(counts.lanes_unframed, 0, "{counts:?}");
}

/// The largest offset the list, scrolling in 300px, its last row's
/// bottom margin and its own bottom padding counted, reaches after
/// `edit`.
fn scrolled_to_end(rows: usize, edit: impl Fn(&List)) -> f32 {
    let css = format!(
        "{ROWS} .list {{ height: 300px; overflow: auto; padding-bottom: 5px; }} \
         .row:last-child {{ margin-bottom: 40px; }}"
    );
    let list = list(rows, &css);
    edit(&list);
    list.update();
    let mut doc = list.doc.borrow_mut();
    doc.scroll_to(list.parent, [0.0, 1.0e6]);
    doc.update(VIEW()).expect("updates");
    doc.scroll_offset(list.parent).expect("scrolls")[1]
}

#[test]
fn a_scrolling_list_reaches_its_last_rows_margin() {
    // When: 100 rows of 20px in 300px, then one more inserted
    let at_first = scrolled_to_end(100, |_| {});
    let inserted = scrolled_to_end(100, |list| _ = list.insert_at(50));

    // Then: the range reaches past the last row's start, its margin and
    // the list's padding (Chromium counts both; Taffy's scroll height
    // leaves them out, so this reads each row where Taffy put it)
    assert_eq!((at_first, inserted), (EXPECTED, EXPECTED + 20.0));
}

/// The range layout gave when it wrote each row's start into its layout
/// (layout-lane PR 4b, measured), above Taffy's scroll height of 1722.
const EXPECTED: f32 = 1740.0;

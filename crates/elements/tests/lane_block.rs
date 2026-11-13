//! A block container is laid out by its lane (`layout/lane_layout.rs`,
//! `docs/plans/splice-layout.md`, section 2): an insert, a remove or a
//! resize in a list asks the lane's children that changed and no
//! other, so the layout calls do not grow with the rows; and each
//! container the lane does not lay out is declined with its reason
//! (`vapors_elements::LaneDecline`). Each case also matches a full
//! update.

mod support;

use support::narrow_list::{List, ROWS, VIEW, list};
use support::oracle::assert_matches_full;
use vapors_dom::Dom;
use vapors_elements::{LaneDecline, UpdateCounts};

/// Layout calls an edit of a list of `rows` makes, after `edit`.
fn calls_after(rows: usize, edit: impl Fn(&List)) -> (usize, UpdateCounts) {
    let list = list(rows, &format!("{ROWS} .tall {{ height: 30px; }}"));
    edit(&list);
    let counts = list.update();
    assert_matches_full(&list.doc, VIEW(), "after the edit");
    (counts.layout_calls, counts)
}

#[test]
fn an_insert_asks_the_new_row_and_a_constant_whatever_the_rows() {
    // When: a row inserted in the middle of 50 rows and of 500
    let (small, counts) = calls_after(50, |list| _ = list.insert_at(25));
    let (large, _) = calls_after(500, |list| _ = list.insert_at(250));

    // Then: the list's lane laid out the new row only
    assert_eq!(small, large, "layout calls grow with the rows");
    assert!(counts.lanes >= 1, "{counts:?}");
    assert_eq!(counts.children_edited, 1);
}

#[test]
fn a_remove_asks_a_constant_whatever_the_rows() {
    let (small, counts) = calls_after(50, |list| list.doc.borrow_mut().remove(list.rows[25]));
    let (large, _) = calls_after(500, |list| list.doc.borrow_mut().remove(list.rows[250]));
    assert_eq!(small, large, "layout calls grow with the rows");
    assert!(counts.lanes >= 1, "{counts:?}");
}

#[test]
fn a_resized_row_asks_itself_and_a_constant_whatever_the_rows() {
    let resize = |at: usize| {
        move |list: &List| {
            list.doc
                .borrow_mut()
                .set_attribute(list.rows[at], "class", "row tall");
        }
    };
    let (small, counts) = calls_after(50, resize(25));
    let (large, _) = calls_after(500, resize(250));
    assert_eq!(small, large, "layout calls grow with the rows");
    assert!(counts.lanes >= 1, "{counts:?}");
}

#[test]
fn the_first_update_lays_body_and_the_list_out_by_lane_and_declines_the_flex_rows() {
    // Given / When: the list's rows are flex containers
    let list = list(3, ROWS);
    let counts = list.doc.borrow_mut().update_counts();

    // Then: body and the list are lanes (html, the root, is neither
    // counted nor laned); each row is declined as a flex container
    assert_eq!(
        counts.lane_decline,
        Some(LaneDecline::FlexLaneNotBuilt),
        "{counts:?}"
    );
    assert!(counts.lanes >= 2, "{counts:?}");
    assert!(counts.lanes_declined >= 3, "{counts:?}");
}

/// The list restyled by `class`, then updated: the counts.
fn restyled_list(css: &str, class: &str) -> UpdateCounts {
    let list = list(3, &format!("{ROWS} {css}"));
    list.doc
        .borrow_mut()
        .set_attribute(list.parent, "class", class);
    let counts = list.update();
    assert_matches_full(&list.doc, VIEW(), class);
    counts
}

#[test]
fn a_flex_column_is_declined_as_the_flex_lane_not_built() {
    let counts = restyled_list(
        ".col { display: flex; flex-direction: column; }",
        "list col",
    );
    assert_eq!(counts.lane_decline, Some(LaneDecline::FlexLaneNotBuilt));
}

#[test]
fn align_content_other_than_normal_or_start_is_declined() {
    let counts = restyled_list(".mid { align-content: center; }", "list mid");
    assert_eq!(counts.lane_decline, Some(LaneDecline::AlignContent));
}

#[test]
fn an_absolute_child_declines_the_list() {
    // Given
    let list = list(3, &format!("{ROWS} .abs {{ position: absolute; top: 0; }}"));

    // When: a row leaves the flow
    list.doc
        .borrow_mut()
        .set_attribute(list.rows[1], "class", "row abs");
    let counts = list.update();

    // Then
    assert_eq!(counts.lane_decline, Some(LaneDecline::OutOfFlowChild));
    assert_matches_full(&list.doc, VIEW(), "absolute row");
}

#[test]
fn an_empty_child_declines_the_list_as_collapsing_through() {
    // Given
    let list = list(3, ROWS);

    // When: an empty block put among the rows
    {
        let mut doc = list.doc.borrow_mut();
        let empty = doc.create_element("div");
        doc.insert_before(list.parent, empty, Some(list.rows[1]));
    }
    let counts = list.update();

    // Then
    assert_eq!(counts.lane_decline, Some(LaneDecline::CollapsesThrough));
    assert_matches_full(&list.doc, VIEW(), "empty child");
}

#[test]
fn a_text_field_is_declined_as_not_a_block() {
    // Given: a text field in a flex row (an input is laid out only as
    // a flex or grid item), laid out once
    let list = list(
        2,
        &format!("{ROWS} input {{ width: 6em; }} .wide {{ width: 9em; }}"),
    );
    let field = {
        let mut doc = list.doc.borrow_mut();
        let field = doc.create_element("input");
        doc.set_attribute(field, "type", "text");
        doc.append(list.rows[1], field);
        field
    };
    list.update();

    // When: the field alone is restyled
    list.doc.borrow_mut().set_attribute(field, "class", "wide");
    let counts = list.update();

    // Then: body, the list and the field are lanes (the field's one
    // child is its run); its row, a flex container, is the one decline
    assert_eq!((counts.lanes, counts.lanes_declined), (3, 1), "{counts:?}");
    assert_eq!(counts.lane_decline, Some(LaneDecline::FlexLaneNotBuilt));
    assert_matches_full(&list.doc, VIEW(), "field restyled");
}

#[cfg(feature = "grid")]
#[test]
fn a_grid_is_declined() {
    let counts = restyled_list(".grid { display: grid; }", "list grid");
    assert_eq!(counts.lane_decline, Some(LaneDecline::Grid));
}

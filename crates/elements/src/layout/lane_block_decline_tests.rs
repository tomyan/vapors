//! The cases the mirror declines, each named, with the rest of the
//! page still answered as Taffy did.

use super::compare::compare;
use super::page::{element, page};
use crate::lane_decline::LaneDecline;

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

    // Then: body is declined on its one layout ask; html and the rows
    // compare
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
    // width known when it is placed; html and the row compare
    assert_eq!(
        compared.declined,
        [LaneDecline::OutOfFlowChild, LaneDecline::WidthUnknown]
    );
    assert_eq!(compared.containers, 2, "{compared:?}");
}

#[test]
fn a_child_wholly_before_a_scroll_containers_origin_declines_it() {
    // Given: a scrolling list whose first row ends above the content
    // box's corner, which Taffy leaves out of the overflow
    let css = "body { margin: 0; } .list { height: 40px; overflow: auto; padding: 2px; } \
               .row { height: 20px; } .gone { margin-top: -30px; }";
    let mut doc = page(css, |doc, body| {
        let list = element(doc, body, "list", None);
        element(doc, list, "row gone", Some("gone"));
        element(doc, list, "row", Some("row"));
    });

    // When
    let compared = compare(&mut doc);

    // Then: the list declines on its layout ask; html, body and the
    // rows compare
    assert_eq!(
        compared.declined,
        [LaneDecline::BeforeScrollOrigin],
        "{compared:?}"
    );
    assert_eq!(compared.containers, 4, "{compared:?}");
}

#[test]
fn a_child_wholly_left_of_a_scroll_containers_origin_declines_it() {
    // Given: a scrolling list whose second row ends left of the content
    // box's corner, which Taffy leaves out of the overflow
    let css = "body { margin: 0; } .list { height: 40px; overflow: auto; padding: 2px; } \
               .row { height: 20px; } .gone { width: 20px; margin-left: -40px; }";
    let mut doc = page(css, |doc, body| {
        let list = element(doc, body, "list", None);
        element(doc, list, "row", Some("row"));
        element(doc, list, "row gone", Some("gone"));
    });

    // When
    let compared = compare(&mut doc);

    // Then: the list declines on its layout ask; html, body and the
    // rows compare
    assert_eq!(
        compared.declined,
        [LaneDecline::BeforeScrollOrigin],
        "{compared:?}"
    );
    assert_eq!(compared.containers, 4, "{compared:?}");
}

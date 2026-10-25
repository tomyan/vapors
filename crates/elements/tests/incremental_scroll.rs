//! A scroll only repaints (F2c S9): no element is restyled, no box laid
//! out, no text laid out again; only the placement moves.

mod support;

use support::scroll_list::{at, list};
use support::text;

#[test]
fn a_scroll_restyles_nothing_and_lays_nothing_out() {
    // Given: a scrolling list with text in its rows, updated twice so
    // everything is kept
    let mut list = list("overflow: auto;");
    for row in list.rows.clone() {
        text(&mut list.doc, row, "a row");
    }
    list.doc.update(at(1.0)).expect("updates");
    list.doc.update(at(1.0)).expect("updates");

    // When
    assert!(list.doc.scroll_by([10.0, 10.0], [0.0, 30.0]));
    list.doc.update(at(1.0)).expect("updates");

    // Then
    assert_eq!(list.doc.elements_restyled(), 0);
    assert_eq!(list.doc.boxes_laid_out(), 0);
    assert_eq!(list.doc.text_layouts_built(), 0);
    assert_eq!(list.doc.scroll_offset(list.container), Some([0.0, 30.0]));
}

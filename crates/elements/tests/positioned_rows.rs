//! The positioned rows page (`support/positioned_rows.rs`, flat-scene
//! W6): a class edit on a positioned row and an insert before the
//! middle row each give what a full update gives, and the page's
//! positioned rows are stacked.

use crate::support;

use support::oracle::assert_matches_full;
use support::positioned_rows::{make_positioned, toggle_positioned_marked};
use support::rows_page::{row_before, rows_page, rows_viewport};
use vapors_dom::Dom;
use vapors_elements::SpliceDecline;

#[test]
fn a_class_edit_on_a_positioned_row_matches_a_full_update() {
    // Given
    let page = rows_page(100);
    make_positioned(&page);

    // When, then: added, and removed
    for case in ["added", "removed"] {
        toggle_positioned_marked(&page);
        page.doc
            .borrow_mut()
            .update(rows_viewport())
            .expect("updates");
        assert_matches_full(&page.doc, rows_viewport(), case);
    }
}

#[test]
fn an_insert_among_positioned_rows_matches_a_full_update() {
    // Given
    let page = rows_page(100);
    make_positioned(&page);

    // When
    let middle = page.rows[50].element;
    let added = row_before(&mut page.doc.borrow_mut(), 100, Some(middle)).element;
    page.doc
        .borrow_mut()
        .update(rows_viewport())
        .expect("updates");

    // Then: inserted, and removed again
    assert_matches_full(&page.doc, rows_viewport(), "inserted");
    page.doc.borrow_mut().remove(added);
    page.doc
        .borrow_mut()
        .update(rows_viewport())
        .expect("updates");
    assert_matches_full(&page.doc, rows_viewport(), "removed");
}

#[test]
fn a_row_inserted_with_the_positioned_class_is_stacked() {
    // Given: the page, whose rule makes `.row.positioned` a stacking
    // context, so its paint order is not tree order
    let page = rows_page(100);
    make_positioned(&page);

    // When: a row given the positioned class put in
    let middle = page.rows[50].element;
    let mut doc = page.doc.borrow_mut();
    let added = row_before(&mut doc, 100, Some(middle)).element;
    doc.set_attribute(added, "class", "row positioned");
    doc.update(rows_viewport()).expect("updates");

    // Then: the splice declines it for its stacking
    let counts = doc.update_counts();
    assert_eq!(counts.splice_decline, Some(SpliceDecline::InsertedStacked));
}

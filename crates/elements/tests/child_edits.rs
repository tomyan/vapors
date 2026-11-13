//! An insert or remove under a list edits the list box's children in
//! place: the new row's boxes are built and put after its previous
//! sibling's box, a removed row's taken out, and the list's other
//! children are not walked (`docs/plans/splice-layout.md`, section 7,
//! PR 1). Where a text run could change, the list is built shallow as
//! before (`child_edits_declined.rs`). Each case also matches a full
//! update.

mod support;

use support::narrow_list::{ROWS, VIEW, list, row};
use support::oracle::assert_matches_full;
use vapors_dom::Dom;

#[test]
fn an_insert_in_the_middle_edits_the_lists_children() {
    // Given
    let list = list(20, ROWS);

    // When
    list.insert_at(10);
    let counts = list.update();

    // Then: the list's box, the row's, its cell's and its run's
    assert_eq!(counts.children_edited, 1);
    assert_eq!(counts.boxes_built, 4);
    assert_matches_full(&list.doc, VIEW(), "insert in the middle");
}

#[test]
fn a_remove_edits_the_lists_children() {
    // Given
    let list = list(20, ROWS);

    // When
    list.doc.borrow_mut().remove(list.rows[10]);
    let counts = list.update();

    // Then
    assert_eq!(counts.children_edited, 1);
    assert_eq!(counts.boxes_built, 1);
    assert_matches_full(&list.doc, VIEW(), "remove in the middle");
}

#[test]
fn inserts_first_and_last_edit_the_lists_children() {
    // Given
    let list = list(5, ROWS);

    // When
    list.insert_at(0);
    {
        let mut doc = list.doc.borrow_mut();
        let appended = row(&mut doc, "appended");
        doc.append(list.parent, appended);
    }
    let counts = list.update();

    // Then
    assert_eq!(counts.children_edited, 1);
    assert_matches_full(&list.doc, VIEW(), "first and last");
}

#[test]
fn rows_appended_in_order_edit_the_lists_children() {
    // Given
    let list = list(3, ROWS);

    // When: three rows appended one after another
    {
        let mut doc = list.doc.borrow_mut();
        for index in 0..3 {
            let added = row(&mut doc, &format!("added {index}"));
            doc.append(list.parent, added);
        }
    }
    let counts = list.update();

    // Then
    assert_eq!(counts.children_edited, 1);
    assert_matches_full(&list.doc, VIEW(), "appended in order");
}

#[test]
fn an_insert_and_a_remove_in_one_update_edit_the_lists_children() {
    // Given
    let list = list(10, ROWS);

    // When
    list.insert_at(7);
    list.doc.borrow_mut().remove(list.rows[2]);
    let counts = list.update();

    // Then
    assert_eq!(counts.children_edited, 1);
    assert_matches_full(&list.doc, VIEW(), "insert and remove");
}

#[test]
fn a_row_inserted_then_removed_before_the_update_changes_nothing_visible() {
    // Given
    let list = list(10, ROWS);

    // When
    let added = list.insert_at(4);
    list.doc.borrow_mut().remove(added);
    let counts = list.update();

    // Then: an edit with nothing in it builds the list's box alone
    assert_eq!(counts.children_edited, 1);
    assert_eq!(counts.boxes_built, 1);
    assert_matches_full(&list.doc, VIEW(), "inserted then removed");
}

#[test]
fn an_inserted_row_with_display_none_edits_nothing_and_builds_no_box() {
    // Given
    let list = list(6, &format!("{ROWS} .gone {{ display: none; }}"));

    // When
    let added = list.insert_at(2);
    list.doc
        .borrow_mut()
        .set_attribute(added, "class", "row gone");
    let counts = list.update();

    // Then
    assert_eq!(counts.children_edited, 1);
    assert_eq!(counts.boxes_built, 1);
    assert_matches_full(&list.doc, VIEW(), "hidden row");
}

#[test]
fn an_insert_after_a_row_rebuilt_in_the_same_update_goes_after_it() {
    // Given
    let list = list(6, &format!("{ROWS} .tall {{ height: 30px; }}"));

    // When: row 2 is restyled, so built again as its own owner, and a
    // row is inserted just after it
    list.doc
        .borrow_mut()
        .set_attribute(list.rows[2], "class", "row tall");
    list.insert_at(3);
    let counts = list.update();

    // Then
    assert_eq!(counts.children_edited, 1);
    assert_matches_full(&list.doc, VIEW(), "insert after a rebuilt row");
}

#[test]
fn a_row_appended_then_moved_before_the_update_edits_the_lists_children() {
    // Given
    let list = list(10, ROWS);

    // When: as the rows workload inserts, appended first, then moved
    {
        let mut doc = list.doc.borrow_mut();
        let added = row(&mut doc, "new");
        doc.append(list.parent, added);
        doc.insert_before(list.parent, added, Some(list.rows[5]));
    }
    let counts = list.update();

    // Then
    assert_eq!(counts.children_edited, 1);
    assert_eq!(counts.boxes_built, 4);
    assert_matches_full(&list.doc, VIEW(), "appended then moved");
}

#[test]
fn a_row_moved_to_another_list_edits_both_lists() {
    // Given: a second list of rows after the first
    let first = list(5, ROWS);
    let second = {
        let mut doc = first.doc.borrow_mut();
        let body = doc.body();
        let second = support::element(&mut doc, body, "div", "list");
        let kept = row(&mut doc, "kept");
        doc.append(second, kept);
        doc.update(VIEW()).expect("updates");
        second
    };

    // When
    first.doc.borrow_mut().append(second, first.rows[2]);
    let counts = first.update();

    // Then: the moved row's boxes are built in the second list
    assert_eq!(counts.children_edited, 2);
    assert_matches_full(&first.doc, VIEW(), "moved between lists");
}

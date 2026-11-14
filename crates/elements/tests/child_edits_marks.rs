//! An edit keeps every box under the list but the inserted and removed
//! children's, so a change elsewhere under the list in the same update
//! builds it shallow instead (review of #285, H1 and M1), and inserted
//! children go in in DOM order whatever order they were inserted in
//! (H2). Each case also matches a full update.

mod support;

use support::narrow_list::{List, ROWS, VIEW, list, row};
use support::oracle::assert_matches_full;
use support::{element, text};
use vapors_dom::{Dom, NodeId};
use vapors_elements::EditDecline;

const MORE: &str = ".hidden { display: none; } .flat { display: inline; } .big { color: red; }";

/// A list of six rows styled by `ROWS` and `MORE`.
fn styled() -> List {
    list(6, &format!("{ROWS} {MORE}"))
}

/// Updates; asserts the list was built shallow because a box under it
/// changed, and that the result matches a full update.
fn assert_marked_inside(list: &List, context: &str) {
    let counts = list.update();
    assert_eq!(counts.children_edited, 0, "{context}");
    assert_eq!(
        counts.children_edit_decline,
        Some(EditDecline::MarkedInside),
        "{context}"
    );
    assert_matches_full(&list.doc, VIEW(), context);
}

#[test]
fn an_inline_restyled_beside_an_insert_builds_the_list_shallow() {
    // Given: a span first in the list, in its own run
    let list = styled();
    let span = {
        let mut doc = list.doc.borrow_mut();
        let span = doc.create_element("span");
        doc.insert_before(list.parent, span, Some(list.rows[0]));
        text(&mut doc, span, "label");
        span
    };
    list.update();

    // When: the span restyled, and a row inserted far from it
    list.doc.borrow_mut().set_attribute(span, "class", "big");
    list.insert_at(4);

    // Then
    assert_marked_inside(&list, "span restyled");
}

#[test]
fn a_row_hidden_in_the_update_of_an_insert_builds_the_list_shallow() {
    let list = styled();
    list.doc
        .borrow_mut()
        .set_attribute(list.rows[5], "class", "row hidden");
    list.insert_at(2);
    assert_marked_inside(&list, "row hidden");
}

#[test]
fn a_row_shown_just_before_an_insert_builds_the_list_shallow() {
    // Given: row 3 hidden
    let list = styled();
    list.doc
        .borrow_mut()
        .set_attribute(list.rows[3], "class", "row hidden");
    list.update();

    // When: shown again, and a row inserted just after it
    list.doc
        .borrow_mut()
        .set_attribute(list.rows[3], "class", "row");
    list.insert_at(4);

    // Then
    assert_marked_inside(&list, "row shown");
}

#[test]
fn a_neighbour_turned_inline_beside_an_insert_builds_the_list_shallow() {
    // Given: a block holding only text between rows 2 and 3
    let list = styled();
    let note = {
        let mut doc = list.doc.borrow_mut();
        let note = doc.create_element("div");
        doc.insert_before(list.parent, note, Some(list.rows[3]));
        text(&mut doc, note, "note");
        note
    };
    list.update();

    // When: it turns inline, and a row is inserted just after it
    list.doc.borrow_mut().set_attribute(note, "class", "flat");
    list.insert_at(3);

    // Then: the check reads the new style, and declines first
    let counts = list.update();
    assert_eq!(counts.children_edit_decline, Some(EditDecline::TextBeside));
    assert_matches_full(&list.doc, VIEW(), "neighbour inline");
}

#[test]
fn a_list_hidden_inside_an_edited_parent_is_built_with_its_parent() {
    // Given: the rows' list under body, and a row in body after it
    let list = styled();
    {
        let mut doc = list.doc.borrow_mut();
        let body = doc.body();
        element(&mut doc, body, "div", "after");
    }
    list.update();

    // When: body gets a row, the list is hidden and gets a row too
    {
        let mut doc = list.doc.borrow_mut();
        let body = doc.body();
        let in_body = row(&mut doc, "in body");
        doc.append(body, in_body);
        doc.set_attribute(list.parent, "class", "list hidden");
        let in_list = row(&mut doc, "in list");
        doc.append(list.parent, in_list);
    }

    // Then
    list.update();
    assert_matches_full(&list.doc, VIEW(), "list hidden");
}

/// Inserts `first` before row `at`, then `second` before `first`.
fn inserted_in_reverse(list: &List, at: usize, second: NodeId) -> NodeId {
    let mut doc = list.doc.borrow_mut();
    let first = row(&mut doc, "first");
    doc.insert_before(list.parent, first, Some(list.rows[at]));
    doc.insert_before(list.parent, second, Some(first));
    first
}

#[test]
fn rows_inserted_in_reverse_order_go_in_in_dom_order() {
    // Given
    let list = styled();
    let second = row(&mut list.doc.borrow_mut(), "second");

    // When: as a keyed `{for}` builds new rows, last first
    inserted_in_reverse(&list, 3, second);
    let counts = list.update();

    // Then
    assert_eq!(counts.children_edited, 1);
    assert_matches_full(&list.doc, VIEW(), "reverse order");
}

#[test]
fn a_row_moved_in_before_a_row_inserted_after_it_goes_in_in_dom_order() {
    // Given: a second list holding one row
    let list = styled();
    let moved = {
        let mut doc = list.doc.borrow_mut();
        let body = doc.body();
        let other = element(&mut doc, body, "div", "list");
        let moved = row(&mut doc, "moved");
        doc.append(other, moved);
        moved
    };
    list.update();

    // When: a row inserted at 3, then the other list's row before it
    inserted_in_reverse(&list, 3, moved);

    // Then
    list.update();
    assert_matches_full(&list.doc, VIEW(), "moved in before");
}

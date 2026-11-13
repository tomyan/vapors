//! Where an insert or remove could change a text run, or a child has
//! no box, the list is built shallow as before rather than edited
//! (`child_edits.rs`, `layout/child_edits.rs`). Each case also matches
//! a full update.

mod support;

use support::narrow_list::{ROWS, VIEW, list};
use support::oracle::assert_matches_full;
use support::text;
use vapors_dom::Dom;

#[test]
fn an_insert_beside_text_builds_the_list_shallow() {
    // Given: a list whose rows have a text node between two of them
    let list = list(6, ROWS);
    {
        let mut doc = list.doc.borrow_mut();
        let words = doc.create_text("words");
        doc.insert_before(list.parent, words, Some(list.rows[3]));
    }
    list.update();

    // When: a row inserted just after the text
    list.insert_at(3);
    let counts = list.update();

    // Then: the run beside it may change, so no edit
    assert_eq!(counts.children_edited, 0);
    assert_matches_full(&list.doc, VIEW(), "insert beside text");
}

#[test]
fn removing_a_row_beside_text_builds_the_list_shallow() {
    // Given: text between rows 2 and 3, and between 3 and 4
    let list = list(6, ROWS);
    {
        let mut doc = list.doc.borrow_mut();
        let parent = list.parent;
        let before = doc.create_text("before");
        doc.insert_before(parent, before, Some(list.rows[3]));
        let after = doc.create_text("after");
        doc.insert_before(parent, after, Some(list.rows[4]));
    }
    list.update();

    // When: removing row 3 joins the two runs into one
    list.doc.borrow_mut().remove(list.rows[3]);
    let counts = list.update();

    // Then
    assert_eq!(counts.children_edited, 0);
    assert_matches_full(&list.doc, VIEW(), "remove between texts");
}

#[test]
fn inserting_text_builds_the_list_shallow() {
    // Given
    let list = list(6, ROWS);

    // When
    text(&mut list.doc.borrow_mut(), list.parent, "tail");
    let counts = list.update();

    // Then
    assert_eq!(counts.children_edited, 0);
    assert_matches_full(&list.doc, VIEW(), "text appended");
}

#[test]
fn a_row_moved_within_the_list_builds_the_list_shallow() {
    // Given
    let list = list(10, ROWS);

    // When: row 7 moved before row 2, so its key holds a slot already
    list.doc
        .borrow_mut()
        .insert_before(list.parent, list.rows[7], Some(list.rows[2]));
    let counts = list.update();

    // Then
    assert_eq!(counts.children_edited, 0);
    assert_matches_full(&list.doc, VIEW(), "move within the list");
}

#[test]
fn removing_a_child_that_has_no_box_builds_the_list_shallow() {
    // Given: a text node between rows 2 and 3
    let list = list(6, ROWS);
    let words = {
        let mut doc = list.doc.borrow_mut();
        let words = doc.create_text("words");
        doc.insert_before(list.parent, words, Some(list.rows[3]));
        words
    };
    list.update();

    // When
    list.doc.borrow_mut().remove(words);
    let counts = list.update();

    // Then
    assert_eq!(counts.children_edited, 0);
    assert_matches_full(&list.doc, VIEW(), "text removed");
}

#[test]
fn an_insert_beside_an_inline_element_builds_the_list_shallow() {
    // Given: an inline span between rows 2 and 3
    let list = list(6, ROWS);
    {
        let mut doc = list.doc.borrow_mut();
        let span = doc.create_element("span");
        doc.insert_before(list.parent, span, Some(list.rows[3]));
        text(&mut doc, span, "inline");
    }
    list.update();

    // When: a row inserted just after the span
    list.insert_at(3);
    let counts = list.update();

    // Then
    assert_eq!(counts.children_edited, 0);
    assert_matches_full(&list.doc, VIEW(), "insert beside inline");
}

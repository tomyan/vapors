//! Scroll containers (F2c S9): `overflow: scroll` and `auto` keep an
//! offset that `Document::scroll_by` at a point moves, which moves their
//! content and what is hit there.

mod support;

use support::scroll_list::{at, list, scroll, top};

#[test]
fn scrolling_moves_the_content_up_by_the_offset() {
    for overflow in ["scroll", "auto"] {
        // Given
        let mut list = list(&format!("overflow: {overflow};"));

        // When
        let scrolled = scroll(&mut list.doc, [0.0, 30.0]);

        // Then
        assert!(scrolled, "{overflow}");
        assert_eq!(top(&mut list.doc, list.rows[1]), 10.0, "{overflow}");
        assert_eq!(
            list.doc.scroll_offset(list.container),
            Some([0.0, 30.0]),
            "{overflow}"
        );
    }
}

#[test]
fn a_hidden_box_is_not_scrolled_by_input() {
    // Given
    let mut list = list("overflow: hidden;");

    // When
    let scrolled = scroll(&mut list.doc, [0.0, 30.0]);

    // Then
    assert!(!scrolled);
    assert_eq!(top(&mut list.doc, list.rows[1]), 40.0);
}

#[test]
fn nothing_scrolls_at_a_point_on_no_scroll_container() {
    // Given
    let mut list = list("overflow: auto;");
    list.doc.update(at(1.0)).expect("updates");

    // When: below the list
    let scrolled = list.doc.scroll_by([10.0, 150.0], [0.0, 30.0]);

    // Then
    assert!(!scrolled);
}

#[test]
fn a_hit_on_scrolled_content_finds_what_is_shown_there() {
    // Given
    let mut list = list("overflow: auto;");
    scroll(&mut list.doc, [0.0, 60.0]);

    // When: row 1 is at -20..20, row 2 at 20..60
    let hit = list.doc.update(at(1.0)).expect("updates").hit([10.0, 30.0]);

    // Then
    assert_eq!(hit, Some(list.rows[2]));
}

#[test]
fn an_offset_is_none_for_an_element_that_does_not_scroll() {
    // Given
    let mut list = list("");
    list.doc.update(at(1.0)).expect("updates");

    // When / Then
    assert_eq!(list.doc.scroll_offset(list.container), None);
    assert_eq!(list.doc.scroll_offset(list.rows[0]), None);
}

#[test]
fn a_scroll_that_moves_marks_the_document_changed() {
    // Given
    let mut list = list("overflow: auto;");
    list.doc.update(at(1.0)).expect("updates");
    list.doc.take_changed();

    // When / Then: a scroll that moves
    assert!(list.doc.scroll_by([10.0, 10.0], [0.0, 30.0]));
    assert!(list.doc.take_changed());

    // When / Then: one up past the start, which moves back to 0, then
    // one that cannot move
    assert!(list.doc.scroll_by([10.0, 10.0], [0.0, -1000.0]));
    assert!(list.doc.take_changed());
    assert!(!list.doc.scroll_by([10.0, 10.0], [0.0, -10.0]));
    assert!(!list.doc.take_changed());
}

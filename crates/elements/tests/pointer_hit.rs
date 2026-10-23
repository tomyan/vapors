//! Hit testing against the last update: edges, `pointer-events: none`,
//! `visibility: hidden`, and nothing before the first update.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::document;
use support::pointer::{SHEET, setup};
use vapors_elements::Document;

#[test]
fn pointer_events_none_lets_the_pointer_through_to_the_element_below() {
    // Given
    let (doc, row, _, _) =
        setup("body { margin: 0; } .a, .b { height: 20px; } .a { pointer-events: none; }");

    // When
    let hit = doc.borrow().hit([5.0, 5.0]);

    // Then
    assert_eq!(hit, Some(row));
}

#[test]
fn a_hidden_element_takes_no_pointer_and_paints_nothing() {
    // Given
    let (doc, row, _, _) = setup(
        "body { margin: 0; } .a, .b { height: 20px; } .a { visibility: hidden; background-color: red; }",
    );

    // When
    let hit = doc.borrow().hit([5.0, 5.0]);

    // Then
    assert_eq!(hit, Some(row));
    assert_eq!(doc.borrow().painted().expect("painted").scene().items(), []);
}

#[test]
fn before_the_first_update_nothing_is_hit() {
    // Given
    let doc = Rc::new(RefCell::new(document()));

    // When
    let hit = doc.borrow().hit([1.0, 1.0]);
    Document::tapped(&doc, [1.0, 1.0]);

    // Then
    assert_eq!(hit, None);
}

#[test]
fn a_box_holds_its_top_left_edges_but_not_its_bottom_right() {
    // Given: .a is y 0..20, .b is y 20..40
    let (doc, _, a, b) = setup(SHEET);

    // When
    let (top, bottom) = (doc.borrow().hit([0.0, 0.0]), doc.borrow().hit([0.0, 20.0]));

    // Then
    assert_eq!((top, bottom), (Some(a), Some(b)));
}

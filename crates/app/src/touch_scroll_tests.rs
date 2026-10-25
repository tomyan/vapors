use super::*;

use crate::scroll_fixture::{nested, offset};

#[test]
fn a_gesture_keeps_the_container_it_first_moved() {
    // Given: a finger down on the inner list
    let (doc, outer, inner) = nested();
    let mut touch = TouchScroll::default();
    touch.down(1, [10.0, 10.0]);

    // When: it pans to the inner list's end, then past it
    touch.pan(&mut doc.borrow_mut(), [0.0, 30.0]);
    touch.pan(&mut doc.borrow_mut(), [0.0, 30.0]);
    touch.pan(&mut doc.borrow_mut(), [0.0, 30.0]);

    // Then: the inner list stops at its end; the outer one never moves
    assert_eq!(offset(&doc, inner), 40.0);
    assert_eq!(offset(&doc, outer), 0.0);
}

#[test]
fn the_next_gesture_finds_its_container_again() {
    // Given: a gesture that left the inner list at its end
    let (doc, outer, inner) = nested();
    let mut touch = TouchScroll::default();
    touch.down(1, [10.0, 10.0]);
    touch.pan(&mut doc.borrow_mut(), [0.0, 50.0]);
    touch.up(1);

    // When: another gesture at the same place
    touch.down(1, [10.0, 10.0]);
    touch.pan(&mut doc.borrow_mut(), [0.0, 30.0]);

    // Then: the outer one takes it
    assert_eq!(offset(&doc, inner), 40.0);
    assert_eq!(offset(&doc, outer), 30.0);
}

#[test]
fn a_pan_without_a_finger_down_scrolls_nothing() {
    // Given
    let (doc, outer, inner) = nested();
    let mut touch = TouchScroll::default();

    // When
    let moved = touch.pan(&mut doc.borrow_mut(), [0.0, 30.0]);

    // Then
    assert!(!moved);
    assert_eq!((offset(&doc, inner), offset(&doc, outer)), (0.0, 0.0));
}

#[test]
fn thumbs_show_with_a_mouse_and_on_touch_only_while_a_gesture_scrolls() {
    // Given
    let (doc, _outer, _inner) = nested();
    let mut touch = TouchScroll::default();
    assert!(touch.thumbs_shown(), "a mouse until a finger touches");

    // When / Then: a finger down, not yet scrolling
    touch.down(1, [10.0, 10.0]);
    assert!(!touch.thumbs_shown());

    // When / Then: the gesture scrolls, then ends
    touch.pan(&mut doc.borrow_mut(), [0.0, 10.0]);
    assert!(touch.thumbs_shown());
    touch.up(1);
    assert!(!touch.thumbs_shown());

    // When / Then: the mouse again
    touch.mouse();
    assert!(touch.thumbs_shown());
}

#[test]
fn a_second_finger_lifting_does_not_end_the_gesture() {
    // Given: two fingers down, the gesture scrolling the inner list
    let (doc, _outer, inner) = nested();
    let mut touch = TouchScroll::default();
    touch.down(1, [10.0, 10.0]);
    touch.down(2, [30.0, 10.0]);
    touch.pan(&mut doc.borrow_mut(), [0.0, 10.0]);

    // When: the second finger lifts, and the first pans on
    touch.up(2);
    touch.pan(&mut doc.borrow_mut(), [0.0, 10.0]);

    // Then: still the same gesture, still scrolling
    assert_eq!(offset(&doc, inner), 20.0);
    assert!(touch.thumbs_shown());

    // When / Then: the last finger lifting ends it
    touch.up(1);
    assert!(!touch.thumbs_shown());
}

#[test]
fn a_finger_lifted_twice_or_never_down_ends_nothing() {
    // Given: one finger down and scrolling
    let (doc, _outer, inner) = nested();
    let mut touch = TouchScroll::default();
    touch.down(1, [10.0, 10.0]);
    touch.pan(&mut doc.borrow_mut(), [0.0, 10.0]);

    // When: a finger that was never down lifts
    touch.up(7);
    touch.pan(&mut doc.borrow_mut(), [0.0, 10.0]);

    // Then
    assert_eq!(offset(&doc, inner), 20.0);
}

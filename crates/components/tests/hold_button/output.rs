//! A `HoldButton`'s hold is its own: consumers receive each change and
//! cannot write it back.
use crate::support::Mounted;
use std::{cell::RefCell, rc::Rc};
use vapors_signals::Owner;

#[test]
fn two_buttons_on_one_consumer_report_their_own_holds() {
    // Given
    let left = Mounted::new();
    let right = Mounted::beside(&left.dom, "right");
    // When
    left.pointer("pointerdown", 1);
    right.pointer("pointerdown", 2);
    left.pointer("pointerup", 1);
    // Then
    assert!(!left.pressed.get());
    assert!(right.pressed.get());
}

#[test]
fn a_consumer_writing_its_signal_does_not_release_the_button() {
    // Given
    let m = Mounted::new();
    m.pointer("pointerdown", 10);
    // When
    m.pressed.set(false);
    // Then
    assert!(m.button.held.get());
    m.pointer("pointerdown", 11);
    m.pointer("pointerup", 10);
    assert!(m.button.held.get(), "pointer 11 still holds the button");
}

#[test]
fn the_consumer_hears_each_change_once() {
    // Given
    let heard = Rc::new(RefCell::new(Vec::new()));
    let on_change: Rc<dyn Fn(bool)> = {
        let heard = Rc::clone(&heard);
        Rc::new(move |held| heard.borrow_mut().push(held))
    };
    let m = Mounted::reporting_to(on_change);
    // When
    m.pointer("pointerdown", 1);
    m.pointer("pointerdown", 2);
    m.pointer("pointerup", 1);
    m.pointer("pointerup", 2);
    // Then
    assert_eq!(*heard.borrow(), [false, true, false]);
}

#[test]
fn disposing_the_consumers_scope_while_held_does_not_panic() {
    // Given
    let parent = Owner::new();
    let m = parent.with(Mounted::new);
    m.pointer("pointerdown", 10);
    // When: the consumer's signal is disposed before the button's cleanup runs
    parent.dispose();
    // Then
    assert!(m.pressed.try_get_untracked().is_err());
}

#[test]
#[should_panic(expected = "HoldButton could not report a hold")]
fn a_hold_into_a_disposed_signal_is_refused() {
    // Given
    let consumer = vapors_signals::Signal::new(false);
    consumer.dispose();
    // When / Then
    vapors_components::hold_into(consumer)(true);
}

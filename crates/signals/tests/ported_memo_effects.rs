//! Ported from `reactive_graph`'s `tests/memo.rs` (leptos-rs/leptos): the
//! tests that pair a memo with an effect.
//!

// The originals' short names are kept so the port is easy to check.
#![allow(clippy::many_single_char_names, clippy::items_after_statements)]

use std::cell::RefCell;
use std::rc::Rc;

use vapors_signals::{Effect, Memo, Owner, Signal};

fn counter() -> Rc<RefCell<i32>> {
    Rc::new(RefCell::new(0))
}
#[test]
fn dynamic_dependencies() {
    // Given
    let owner = Owner::new();
    owner.set();
    let first = Signal::new("Greg");
    let last = Signal::new("Johnston");
    let use_last = Signal::new(true);
    let name = Memo::new(move || {
        if use_last.get() {
            format!("{} {}", first.get(), last.get())
        } else {
            first.get().to_string()
        }
    });
    let combined_count = counter();
    Effect::new({
        let combined_count = Rc::clone(&combined_count);
        move || {
            let _ = name.get();
            *combined_count.borrow_mut() += 1;
        }
    });
    assert_eq!(*combined_count.borrow(), 1);

    // When / Then
    first.set("Bob");
    assert_eq!(name.get_untracked(), "Bob Johnston");
    assert_eq!(*combined_count.borrow(), 2);

    last.set("Thompson");
    assert_eq!(*combined_count.borrow(), 3);

    use_last.set(false);
    assert_eq!(name.get_untracked(), "Bob");
    assert_eq!(*combined_count.borrow(), 4);

    last.set("Jones");
    last.set("Smith");
    last.set("Stevens");
    assert_eq!(*combined_count.borrow(), 4);

    use_last.set(true);
    assert_eq!(name.get_untracked(), "Bob Stevens");
    assert_eq!(*combined_count.borrow(), 5);
}

#[test]
fn effect_doesnt_rerun_if_memo_didnt_change() {
    // Given
    let owner = Owner::new();
    owner.set();
    let count = Signal::new(1);
    let even = Memo::new(move || count.get() % 2 == 0);
    let combined_count = counter();
    Effect::new({
        let combined_count = Rc::clone(&combined_count);
        move || {
            *combined_count.borrow_mut() += 1;
            let _ = even.get();
        }
    });
    assert_eq!(*combined_count.borrow(), 1);

    // When / Then
    count.set(2);
    assert_eq!(*combined_count.borrow(), 2);
    count.set(4);
    assert_eq!(*combined_count.borrow(), 2);
}

#[test]
fn effect_depending_on_signal_and_memo_doesnt_rerun_unnecessarily() {
    // Given
    let owner = Owner::new();
    owner.set();
    let other_signal = Signal::new(false);
    let count = Signal::new(1);
    let even = Memo::new(move || count.get() % 2 == 0);
    let combined_count = counter();
    Effect::new({
        let combined_count = Rc::clone(&combined_count);
        move || {
            *combined_count.borrow_mut() += 1;
            let _ = (even.get(), other_signal.get());
        }
    });
    assert_eq!(*combined_count.borrow(), 1);

    // When / Then
    count.set(2);
    assert_eq!(*combined_count.borrow(), 2);
    count.set(4);
    assert_eq!(*combined_count.borrow(), 2);
}

//! Ported from `reactive_graph`'s `tests/cleanup.rs` (leptos-rs/leptos).
//! `Trigger` is a unit signal used only to give the memo a dependency.

// The originals' short names are kept so the port is easy to check.
#![allow(clippy::many_single_char_names, clippy::items_after_statements)]

use std::rc::Rc;

use vapors_signals::{Memo, Owner, Signal, Trigger, on_cleanup};

#[test]
fn cleanup_on_dispose() {
    // Given
    let owner = Owner::new();
    owner.set();

    struct ExecuteOnDrop(Option<Box<dyn FnOnce()>>);
    impl Drop for ExecuteOnDrop {
        fn drop(&mut self) {
            self.0.take().unwrap()();
        }
    }

    let trigger = Trigger::new();
    let memo = Memo::new(move || {
        trigger.track();
        // Cleaning up may itself register a cleanup; that must not panic.
        let on_drop = ExecuteOnDrop(Some(Box::new(|| on_cleanup(|| ()))));
        on_cleanup(move || drop(on_drop));
    });
    memo.with_untracked(|()| ());

    // When
    memo.dispose();

    // Then: a fresh memo can register cleanups after the first was disposed.
    let memo = Memo::new(move || on_cleanup(|| ()));
    memo.with_untracked(|()| ());
    memo.dispose();
}

#[test]
fn leak_on_dispose() {
    // Given
    let owner = Owner::new();
    owner.set();
    let trigger = Trigger::new();
    let value = Rc::new(());
    let weak = Rc::downgrade(&value);
    let memo = Memo::new(move || {
        trigger.track();
        Signal::new(Rc::clone(&value));
    });
    memo.with_untracked(|()| ());

    // When
    memo.dispose();

    // Then
    assert!(weak.upgrade().is_none());
}

//! Ported from `reactive_graph`'s `tests/memo.rs` (leptos-rs/leptos).
//!
//! Effects here are synchronous (there is no executor to tick), so the
//! `Executor::tick().await` lines of the originals are simply gone; the
//! assertions are unchanged. Not ported, and why:
//! - `arc_memo_readable`: no `Arc` variant on a single-threaded runtime.
//! - `unsync_derived_signal_and_memo`: `Signal::derive_local` has no
//!   counterpart here; derived values are plain closures over `get()`.
//! - `test_memo_multiple_read_guards`, `test_memo_read_guard_held`:
//!   there are no read guards.
//! - `sync_effect_can_recompute_dirty_memo_while_being_notified`: covered
//!   by `effect_doesnt_rerun_if_memo_didnt_change` since every effect here
//!   is synchronous.

// The originals' short names are kept so the port is easy to check.
#![allow(clippy::many_single_char_names, clippy::items_after_statements)]

use std::cell::RefCell;
use std::rc::Rc;

use vapors_signals::{Memo, Owner, Signal};

fn counter() -> Rc<RefCell<i32>> {
    Rc::new(RefCell::new(0))
}

#[test]
fn memo_calculates_value() {
    // Given
    let owner = Owner::new();
    owner.set();
    let a = Signal::new(1);
    let b = Signal::new(2);
    let c = Signal::new(3);

    // When
    let d = Memo::new(move || a.get() + b.get() + c.get());

    // Then
    assert_eq!(d.with_untracked(|n| *n), 6);
    assert_eq!(d.with(|n| *n), 6);
    assert_eq!(d.get_untracked(), 6);
}

#[test]
fn memo_doesnt_repeat_calculation_per_get() {
    // Given
    let owner = Owner::new();
    owner.set();
    let calculations = counter();
    let a = Signal::new(1);
    let b = Signal::new(2);
    let c = Signal::new(3);
    let d = Memo::new({
        let calculations = Rc::clone(&calculations);
        move || {
            *calculations.borrow_mut() += 1;
            a.get() + b.get() + c.get()
        }
    });

    // When / Then
    assert_eq!(d.get_untracked(), 6);
    assert_eq!(d.get_untracked(), 6);
    assert_eq!(d.get_untracked(), 6);
    assert_eq!(*calculations.borrow(), 1);

    a.set(0);
    assert_eq!(d.get_untracked(), 5);
    assert_eq!(*calculations.borrow(), 2);
}

#[test]
fn nested_memos() {
    // Given
    let owner = Owner::new();
    owner.set();
    let a = Signal::new(0);
    let b = Signal::new(0);
    let c = Memo::new(move || a.get() + b.get());
    let d = Memo::new(move || c.get() * 2);
    let e = Memo::new(move || d.get() + 1);
    assert_eq!(e.get_untracked(), 1);
    assert_eq!(d.get_untracked(), 0);
    assert_eq!(c.get_untracked(), 0);

    // When / Then
    a.set(5);
    assert_eq!(c.get_untracked(), 5);
    assert_eq!(d.get_untracked(), 10);
    assert_eq!(e.get_untracked(), 11);

    b.set(1);
    assert_eq!(e.get_untracked(), 13);
    assert_eq!(d.get_untracked(), 12);
    assert_eq!(c.get_untracked(), 6);
}

#[test]
fn memo_runs_only_when_inputs_change() {
    // Given
    let owner = Owner::new();
    owner.set();
    let call_count = counter();
    let a = Signal::new(0);
    let b = Signal::new(0);
    let c = Signal::new(0);
    let c = Memo::new({
        let call_count = Rc::clone(&call_count);
        move || {
            *call_count.borrow_mut() += 1;
            a.get() + b.get() + c.get()
        }
    });

    // Then: lazy, so not yet called
    assert_eq!(*call_count.borrow(), 0);

    for _ in 0..5 {
        assert_eq!(c.get_untracked(), 0);
    }
    assert_eq!(*call_count.borrow(), 1);

    a.set(1);
    assert_eq!(c.get_untracked(), 1);
    assert_eq!(*call_count.borrow(), 2);
}

#[test]
fn diamond_problem() {
    // Given
    let owner = Owner::new();
    owner.set();
    let name = Signal::new("Greg Johnston".to_string());
    let first = Memo::new(move || name.get().split_whitespace().next().unwrap().to_string());
    let last = Memo::new(move || name.get().split_whitespace().nth(1).unwrap().to_string());
    let combined_count = counter();
    let combined = Memo::new({
        let combined_count = Rc::clone(&combined_count);
        move || {
            *combined_count.borrow_mut() += 1;
            format!("{} {}", first.get(), last.get())
        }
    });
    assert_eq!(first.get_untracked(), "Greg");
    assert_eq!(last.get_untracked(), "Johnston");

    // When
    name.set("Will Smith".to_string());

    // Then
    assert_eq!(first.get_untracked(), "Will");
    assert_eq!(last.get_untracked(), "Smith");
    assert_eq!(combined.get_untracked(), "Will Smith");
    assert_eq!(*combined_count.borrow(), 1);
}

#[test]
fn memo_updates_even_if_not_read_until_later() {
    // Given (regression test for leptos-rs/leptos#3339)
    let owner = Owner::new();
    owner.set();
    let input = Signal::new(0);
    let first_memo = Memo::new(move || input.get() == 1);
    let second_memo = Memo::new(move || first_memo.get());
    assert_eq!(input.get(), 0);
    assert!(!first_memo.get());

    // When / Then
    input.set(1);
    assert!(first_memo.get());
    assert!(second_memo.get());

    input.set(2);
    assert!(!first_memo.get());

    input.set(3);
    assert!(!first_memo.get());
    assert!(!second_memo.get());
}

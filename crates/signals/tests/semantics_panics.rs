//! A panic in user code (inside a batch, an effect, a memo, `untrack`
//! or `Owner::with`) unwinds without leaving the thread's runtime
//! half-changed: what runs after `catch_unwind` behaves as before.

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use vapors_signals::{Effect, Memo, Owner, Signal, batch, untrack};

fn counting_effect(signal: Signal<i32>) -> Rc<Cell<u32>> {
    let runs = Rc::new(Cell::new(0));
    let counter = Rc::clone(&runs);
    Effect::new(move || {
        let _ = signal.get();
        counter.set(counter.get() + 1);
    });
    runs
}

#[test]
fn after_a_panic_inside_a_batch_effects_run_on_the_next_set() {
    // Given
    let owner = Owner::new();
    owner.set();
    let signal = Signal::new(0);
    let runs = counting_effect(signal);

    // When
    let caught = catch_unwind(AssertUnwindSafe(|| batch(|| panic!("inside the batch"))));
    signal.set(1);

    // Then
    assert!(caught.is_err());
    assert_eq!(runs.get(), 2);
}

#[test]
fn after_an_effect_panics_the_effects_queued_behind_it_run_on_the_next_set() {
    // Given
    let owner = Owner::new();
    owner.set();
    let signal = Signal::new(0);
    Effect::new(move || assert!(signal.get() != 1, "the effect refuses one"));
    let runs = counting_effect(signal);

    // When
    let caught = catch_unwind(AssertUnwindSafe(|| signal.set(1)));
    signal.set(2);

    // Then
    assert!(caught.is_err());
    assert_eq!(
        runs.get(),
        2,
        "ran for 0 and for 2; the pass for 1 stopped at the panic"
    );
}

#[test]
fn a_memo_that_panicked_is_computed_again_rather_than_seen_as_a_cycle() {
    // Given
    let owner = Owner::new();
    owner.set();
    let fail = Rc::new(Cell::new(true));
    let flag = Rc::clone(&fail);
    let memo = Memo::new(move || {
        assert!(!flag.get(), "the memo fails once");
        7
    });

    // When
    let caught = catch_unwind(AssertUnwindSafe(|| memo.get()));
    fail.set(false);

    // Then
    assert!(caught.is_err());
    assert_eq!(memo.try_get(), Ok(7));
}

#[test]
fn after_a_panic_inside_owner_with_the_previous_owner_is_current() {
    // Given
    let outer = Owner::new();
    outer.set();
    let inner = Owner::new();

    // When
    let caught = catch_unwind(AssertUnwindSafe(|| inner.with(|| panic!("inside with"))));

    // Then
    assert!(caught.is_err());
    assert_eq!(Owner::current(), Some(outer));
}

#[test]
fn a_panic_caught_inside_untrack_leaves_the_effect_off_the_tracking_stack() {
    // Given: an effect that catches a panic from inside `untrack`
    let owner = Owner::new();
    owner.set();
    let other = Signal::new(0);
    let runs = Rc::new(Cell::new(0));
    let counter = Rc::clone(&runs);
    Effect::new(move || {
        counter.set(counter.get() + 1);
        let _ = catch_unwind(|| untrack(|| panic!("inside untrack")));
    });

    // When: a read outside any effect, then a write to what was read
    let _ = other.get();
    other.set(1);

    // Then: the effect did not read `other`, so it did not run again
    assert_eq!(runs.get(), 1);
}

#[test]
fn an_effect_that_panicked_runs_again_when_what_it_read_changes() {
    // Given
    let owner = Owner::new();
    owner.set();
    let signal = Signal::new(0);
    let runs = Rc::new(Cell::new(0));
    let counter = Rc::clone(&runs);
    Effect::new(move || {
        counter.set(counter.get() + 1);
        assert!(signal.get() != 1, "the effect refuses one");
    });

    // When
    let caught = catch_unwind(AssertUnwindSafe(|| signal.set(1)));
    signal.set(2);

    // Then
    assert!(caught.is_err());
    assert_eq!(runs.get(), 3);
}

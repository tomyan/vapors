//! `set_if_changed`: a write that notifies only when the value differs.

use std::cell::Cell;
use std::rc::Rc;

use vapors_signals::{Effect, Error, Memo, Owner, Signal};

/// An effect running `read`, and the count of its runs.
fn count_runs(read: impl Fn() + 'static) -> Rc<Cell<u32>> {
    let runs = Rc::new(Cell::new(0));
    Effect::new({
        let runs = Rc::clone(&runs);
        move || {
            read();
            runs.set(runs.get() + 1);
        }
    });
    runs
}

#[test]
fn an_equal_value_notifies_no_reader_and_returns_false() {
    // Given
    let owner = Owner::new();
    owner.set();
    let speed = Signal::new(3);
    let runs = count_runs(move || {
        let _ = speed.get();
    });

    // When
    let notified = speed.set_if_changed(3);

    // Then
    assert!(!notified);
    assert_eq!(runs.get(), 1);
}

#[test]
fn a_different_value_notifies_every_reader_and_returns_true() {
    // Given
    let owner = Owner::new();
    owner.set();
    let speed = Signal::new(3);
    let first = count_runs(move || {
        let _ = speed.get();
    });
    let second = count_runs(move || {
        let _ = speed.get();
    });
    let doubled = Memo::new(move || speed.get() * 2);
    let memo_reader = count_runs(move || {
        let _ = doubled.get();
    });

    // When
    let notified = speed.set_if_changed(4);

    // Then
    assert!(notified);
    assert_eq!((first.get(), second.get(), memo_reader.get()), (2, 2, 2));
    assert_eq!(speed.get_untracked(), 4);
}

#[test]
fn a_nan_value_notifies_on_every_write() {
    // Given
    let owner = Owner::new();
    owner.set();
    let level = Signal::new(f64::NAN);
    let runs = count_runs(move || {
        let _ = level.get();
    });

    // When
    let notified = level.set_if_changed(f64::NAN);

    // Then
    assert!(notified);
    assert_eq!(runs.get(), 2);
}

#[test]
fn set_with_an_equal_value_still_notifies() {
    // Given
    let owner = Owner::new();
    owner.set();
    let speed = Signal::new(3);
    let runs = count_runs(move || {
        let _ = speed.get();
    });

    // When
    speed.set(3);

    // Then
    assert_eq!(runs.get(), 2);
}

#[test]
fn comparing_inside_an_effect_does_not_subscribe_it() {
    // Given
    let owner = Owner::new();
    owner.set();
    let source = Signal::new(1);
    let target = Signal::new(0);
    let runs = Rc::new(Cell::new(0));
    Effect::new({
        let runs = Rc::clone(&runs);
        move || {
            target.set_if_changed(source.get());
            runs.set(runs.get() + 1);
        }
    });

    // When
    target.set(7);

    // Then
    assert_eq!(runs.get(), 1);
}

#[test]
fn a_disposed_signal_is_refused_by_name() {
    // Given
    let owner = Owner::new();
    owner.set();
    let speed = Signal::named("speed", 3);
    speed.dispose();

    // When
    let refused = speed.try_set_if_changed(4);

    // Then
    assert!(matches!(refused, Err(Error::Disposed { .. })));
}

//! `Signal::derived`: a signal that follows what its closure reads.

use std::cell::Cell;
use std::rc::Rc;

use vapors_signals::{Effect, Owner, ReadSignal, Signal};

/// A derived `source * 10` and the count of times its closure ran.
fn tenfold(source: Signal<i32>) -> (ReadSignal<i32>, Rc<Cell<u32>>) {
    let runs = Rc::new(Cell::new(0));
    let counted = runs.clone();
    let derived = Signal::derived(move || {
        counted.set(counted.get() + 1);
        source.get() * 10
    });
    (derived, runs)
}

#[test]
fn starts_at_the_closures_value() {
    let owner = Owner::new();
    owner.with(|| {
        let source = Signal::new(4);
        let (derived, _) = tenfold(source);
        assert_eq!(derived.get_untracked(), 40);
    });
}

#[test]
fn follows_a_read_signal_and_only_that() {
    let owner = Owner::new();
    owner.with(|| {
        let (source, other) = (Signal::new(1), Signal::new(0));
        let (derived, runs) = tenfold(source);
        let before = runs.get();
        other.set(5);
        assert_eq!(runs.get(), before, "an unread signal must not re-run it");
        source.set(2);
        assert_eq!(derived.get_untracked(), 20);
        assert_eq!(runs.get(), before + 1, "one write, one run");
    });
}

#[test]
fn disposing_the_owner_stops_the_updates() {
    let source = Signal::new(1);
    let inner = Owner::new();
    let (derived, runs) = inner.with(|| tenfold(source));
    inner.dispose();
    let after_dispose = runs.get();
    source.set(2);
    assert_eq!(runs.get(), after_dispose, "a disposed effect must not run");
    assert!(derived.try_get_untracked().is_err());
}

#[test]
fn the_initial_value_does_not_subscribe_the_creating_effect() {
    let owner = Owner::new();
    owner.with(|| {
        let source = Signal::new(1);
        let creations = Rc::new(Cell::new(0));
        let counted = creations.clone();
        Effect::new(move || {
            counted.set(counted.get() + 1);
            tenfold(source);
        });
        source.set(2);
        assert_eq!(creations.get(), 1, "the creating effect must not re-run");
    });
}

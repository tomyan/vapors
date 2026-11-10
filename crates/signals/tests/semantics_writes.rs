//! What a write notifies, and the order cleanups run in: characterisation
//! of today's behaviour, recorded in docs/design/incremental-contracts.md.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use vapors_signals::{Effect, Memo, Owner, Signal, on_cleanup};

/// An effect reading `signal`, and how many times it has run.
fn counting_effect(signal: Signal<i32>) -> Rc<Cell<u32>> {
    let runs = Rc::new(Cell::new(0));
    Effect::new({
        let runs = Rc::clone(&runs);
        move || {
            let _ = signal.get();
            runs.set(runs.get() + 1);
        }
    });
    runs
}

#[test]
fn setting_an_equal_value_reruns_the_readers() {
    // Given
    let owner = Owner::new();
    owner.set();
    let a = Signal::new(1);
    let runs = counting_effect(a);

    // When
    a.set(1);

    // Then
    assert_eq!(runs.get(), 2);
}

#[test]
fn an_update_that_changes_nothing_reruns_the_readers() {
    // Given
    let owner = Owner::new();
    owner.set();
    let a = Signal::new(1);
    let runs = counting_effect(a);

    // When
    a.update(|_| {});

    // Then
    assert_eq!(runs.get(), 2);
}

#[test]
fn a_memo_of_nan_notifies_its_readers_on_every_recompute() {
    // Given: NaN != NaN, so the memo's PartialEq guard never holds
    let owner = Owner::new();
    owner.set();
    let a = Signal::new(0);
    let nan = Memo::new(move || f64::from(a.get()) * f64::NAN);
    let runs = Rc::new(Cell::new(0));
    Effect::new({
        let runs = Rc::clone(&runs);
        move || {
            let _ = nan.get();
            runs.set(runs.get() + 1);
        }
    });

    // When
    a.set(1);
    a.set(2);

    // Then
    assert_eq!(runs.get(), 3);
}

#[test]
fn a_rerun_runs_the_effects_cleanups_in_order_then_its_nested_effects_newest_first() {
    // Given
    let owner = Owner::new();
    owner.set();
    let a = Signal::new(0);
    let events = Rc::new(RefCell::new(Vec::<String>::new()));
    let push = |events: &Rc<RefCell<Vec<String>>>, text: &'static str| {
        let events = Rc::clone(events);
        on_cleanup(move || events.borrow_mut().push(text.into()));
    };
    Effect::new({
        let events = Rc::clone(&events);
        move || {
            let _ = a.get();
            push(&events, "outer first");
            push(&events, "outer second");
            let inner_events = Rc::clone(&events);
            Effect::new(move || push(&inner_events, "inner one"));
            let inner_events = Rc::clone(&events);
            Effect::new(move || push(&inner_events, "inner two"));
        }
    });

    // When
    a.set(1);

    // Then
    assert_eq!(
        *events.borrow(),
        ["outer first", "outer second", "inner two", "inner one"]
    );
}

#[test]
#[should_panic(expected = "effects re-dirtied themselves 100 passes in a row: effect `forever`")]
fn an_effect_that_rewrites_its_own_source_forever_panics_naming_the_effect() {
    // Given
    let owner = Owner::new();
    owner.set();
    let a = Signal::new(0);

    // When
    Effect::named("forever", move || a.set(a.get() + 1));
}

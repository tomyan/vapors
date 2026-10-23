//! Disposal, untracked reads, and refusals that name the node instead of
//! panicking.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_signals::{Effect, Error, Memo, Owner, Signal, untrack};

fn log() -> Rc<RefCell<Vec<String>>> {
    Rc::new(RefCell::new(Vec::new()))
}
#[test]
fn disposing_an_owner_stops_its_effects() {
    // Given
    let root = Owner::new();
    root.set();
    let a = Signal::new(0);
    let runs = Rc::new(RefCell::new(0));
    let child = Owner::new();
    child.with(|| {
        let runs = Rc::clone(&runs);
        Effect::new(move || {
            let _ = a.get();
            *runs.borrow_mut() += 1;
        });
    });
    assert_eq!(*runs.borrow(), 1);

    // When
    child.dispose();
    a.set(1);

    // Then
    assert_eq!(*runs.borrow(), 1);
}

#[test]
fn disposing_an_owner_disposes_its_child_owners() {
    // Given
    let root = Owner::new();
    root.set();
    let parent = Owner::new();
    let inner = parent.with(|| {
        let child = Owner::new();
        child.with(|| Signal::named("inner", 1))
    });
    assert_eq!(inner.try_get(), Ok(1));

    // When
    parent.dispose();

    // Then
    assert!(matches!(inner.try_get(), Err(Error::Disposed { .. })));
}

#[test]
fn reading_a_disposed_signal_is_refused_by_name() {
    // Given
    let root = Owner::new();
    root.set();
    let owner = Owner::new();
    let count = owner.with(|| Signal::named("count", 1));
    owner.dispose();

    // When
    let err = count.try_get().unwrap_err();

    // Then
    let message = err.to_string();
    assert!(message.contains("count"), "{message}");
    assert!(message.contains("disposed"), "{message}");
    assert!(
        message.contains("semantics_refusals.rs"),
        "names the creation site: {message}"
    );
    assert!(matches!(count.try_set(2), Err(Error::Disposed { .. })));
}

#[test]
fn a_memo_cycle_is_detected_and_refused_by_name() {
    // Given: two memos that read each other through a shared cell
    let owner = Owner::new();
    owner.set();
    let seen: Rc<RefCell<Option<Error>>> = Rc::new(RefCell::new(None));
    let b_cell: Rc<RefCell<Option<Memo<i32>>>> = Rc::new(RefCell::new(None));
    let a = Memo::named("a", {
        let b_cell = Rc::clone(&b_cell);
        move || b_cell.borrow().map_or(0, |b| b.get_untracked())
    });
    let b = Memo::named("b", {
        let seen = Rc::clone(&seen);
        move || match a.try_get_untracked() {
            Ok(value) => value,
            Err(err) => {
                *seen.borrow_mut() = Some(err);
                -1
            }
        }
    });
    *b_cell.borrow_mut() = Some(b);

    // When
    let value = a.get_untracked();

    // Then
    assert_eq!(value, -1);
    let err = seen
        .borrow()
        .clone()
        .expect("the inner read must be refused");
    let message = err.to_string();
    assert!(matches!(err, Error::Cycle { .. }), "{message}");
    assert!(
        message.contains("`a`") && message.contains("`b`"),
        "{message}"
    );
}

#[test]
fn untrack_reads_without_subscribing() {
    // Given
    let owner = Owner::new();
    owner.set();
    let tracked = Signal::new(0);
    let hidden = Signal::new(0);
    let runs = Rc::new(RefCell::new(0));
    Effect::new({
        let runs = Rc::clone(&runs);
        move || {
            let _ = tracked.get();
            let _ = untrack(|| hidden.get());
            *runs.borrow_mut() += 1;
        }
    });

    // When
    hidden.set(1);
    tracked.set(1);

    // Then
    assert_eq!(*runs.borrow(), 2);
}

#[test]
fn disposing_an_effect_stops_it_and_runs_its_cleanups() {
    // Given
    let owner = Owner::new();
    owner.set();
    let a = Signal::new(0);
    let events = log();
    let effect = Effect::new({
        let events = Rc::clone(&events);
        move || {
            let value = a.get();
            events.borrow_mut().push(format!("run {value}"));
            let events = Rc::clone(&events);
            vapors_signals::on_cleanup(move || events.borrow_mut().push(format!("clean {value}")));
        }
    });

    // When
    a.set(1);
    effect.dispose();
    a.set(2);

    // Then
    assert_eq!(*events.borrow(), ["run 0", "clean 0", "run 1", "clean 1"]);
}

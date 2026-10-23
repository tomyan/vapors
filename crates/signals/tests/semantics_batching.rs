//! Glitch-free batching, deterministic effect order and deferred writes.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_signals::{Effect, Memo, Owner, Signal, batch};

fn log() -> Rc<RefCell<Vec<String>>> {
    Rc::new(RefCell::new(Vec::new()))
}

#[test]
fn memo_reading_two_signals_set_in_one_batch_runs_once() {
    // Given
    let owner = Owner::new();
    owner.set();
    let a = Signal::new(1);
    let b = Signal::new(2);
    let runs = Rc::new(RefCell::new(0));
    let sum = Memo::new({
        let runs = Rc::clone(&runs);
        move || {
            *runs.borrow_mut() += 1;
            a.get() + b.get()
        }
    });
    Effect::new(move || {
        let _ = sum.get();
    });
    assert_eq!(*runs.borrow(), 1);

    // When
    batch(|| {
        a.set(10);
        b.set(20);
    });

    // Then
    assert_eq!(sum.get_untracked(), 30);
    assert_eq!(*runs.borrow(), 2);
}

#[test]
fn effects_run_after_the_batch_in_creation_order() {
    // Given
    let owner = Owner::new();
    owner.set();
    let a = Signal::new(0);
    let order = log();
    for name in ["first", "second", "third"] {
        let order = Rc::clone(&order);
        Effect::new(move || {
            let _ = a.get();
            order.borrow_mut().push(name.to_string());
        });
    }
    order.borrow_mut().clear();

    // When
    batch(|| {
        a.set(1);
        assert!(order.borrow().is_empty(), "effects must wait for the batch");
    });

    // Then
    assert_eq!(*order.borrow(), ["first", "second", "third"]);
}

#[test]
fn signal_set_inside_an_effect_is_deferred_until_the_pass_ends() {
    // Given
    let owner = Owner::new();
    owner.set();
    let a = Signal::new(0);
    let b = Signal::new(0);
    let order = log();
    Effect::new({
        let order = Rc::clone(&order);
        move || {
            order
                .borrow_mut()
                .push(format!("writer sees a={}", a.get()));
            if a.get() == 1 {
                b.set(1);
                order.borrow_mut().push("writer set b".into());
            }
        }
    });
    Effect::new({
        let order = Rc::clone(&order);
        move || {
            order
                .borrow_mut()
                .push(format!("reader sees b={}", b.get()));
        }
    });
    order.borrow_mut().clear();

    // When
    a.set(1);

    // Then: the reader runs after the writer has finished, not inside it.
    assert_eq!(
        *order.borrow(),
        ["writer sees a=1", "writer set b", "reader sees b=1"]
    );
}

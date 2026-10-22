//! Ported from `reactive_graph`'s `tests/effect.rs` and
//! `tests/effect_immediate.rs` (leptos-rs/leptos).
//!
//! Every effect here runs synchronously at creation and after each
//! signal write (or at the end of the outermost batch), which is the
//! `ImmediateEffect` behaviour; `RenderEffect` and the executor-driven
//! `Effect` collapse into it, so each test appears once. Not ported:
//! - `paused_effect_pauses`: owners cannot pause; dispose is the only stop.
//! - `threaded_chaos_effect`: ignored upstream; no threads here.
//! - `async_render_effect_cleans_up_its_first_value_on_rerun`: async
//!   values are out of scope.
//! - `watch.rs` entirely: `Effect::watch` is not part of this API.

// The originals' short names are kept so the port is easy to check.
#![allow(clippy::many_single_char_names, clippy::items_after_statements)]

use std::cell::RefCell;
use std::rc::Rc;

use vapors_signals::{Effect, Owner, Signal, batch};

#[test]
fn effect_runs() {
    // Given
    let owner = Owner::new();
    owner.set();
    let a = Signal::new(-1);
    let b = Rc::new(RefCell::new(String::new()));

    // When
    Effect::new({
        let b = Rc::clone(&b);
        move || *b.borrow_mut() = format!("Value is {}", a.get())
    });

    // Then
    assert_eq!(b.borrow().as_str(), "Value is -1");
    a.set(1);
    assert_eq!(b.borrow().as_str(), "Value is 1");
}

#[test]
fn dynamic_dependencies() {
    // Given
    let owner = Owner::new();
    owner.set();
    let first = Signal::new("Greg");
    let last = Signal::new("Johnston");
    let use_last = Signal::new(true);
    let combined_count = Rc::new(RefCell::new(0));
    Effect::new({
        let combined_count = Rc::clone(&combined_count);
        move || {
            *combined_count.borrow_mut() += 1;
            if use_last.get() {
                let _ = (first.get(), last.get());
            } else {
                let _ = first.get();
            }
        }
    });
    assert_eq!(*combined_count.borrow(), 1);

    // When / Then
    first.set("Bob");
    assert_eq!(*combined_count.borrow(), 2);
    last.set("Thompson");
    assert_eq!(*combined_count.borrow(), 3);
    use_last.set(false);
    assert_eq!(*combined_count.borrow(), 4);
    last.set("Jones");
    last.set("Smith");
    last.set("Stevens");
    assert_eq!(*combined_count.borrow(), 4);
    use_last.set(true);
    assert_eq!(*combined_count.borrow(), 5);
}

#[test]
fn recursive_effect_runs_recursively() {
    // Given
    let owner = Owner::new();
    owner.set();
    let s = Signal::new(0);
    let logged_values = Rc::new(RefCell::new(Vec::new()));
    Effect::new({
        let logged_values = Rc::clone(&logged_values);
        move || {
            let a = s.get();
            logged_values.borrow_mut().push(a);
            if a == 0 {
                return;
            }
            s.set(0);
        }
    });

    // When
    s.set(1);
    s.set(2);
    s.set(3);

    // Then
    assert_eq!(0, s.get_untracked());
    assert_eq!(&*logged_values.borrow(), &[0, 1, 0, 2, 0, 3, 0]);
}

#[test]
fn test_batch() {
    // Given
    let owner = Owner::new();
    owner.set();
    let a = Signal::new(0);
    let b = Signal::new(0);
    let values = Rc::new(RefCell::new(Vec::new()));
    Effect::new({
        let values = Rc::clone(&values);
        move || values.borrow_mut().push((a.get(), b.get()))
    });

    // When
    a.set(1);
    b.set(1);
    batch(|| {
        a.set(2);
        b.set(2);
        batch(|| {
            a.set(3);
            b.set(3);
        });
    });

    // Then
    assert_eq!(*values.borrow(), vec![(0, 0), (1, 0), (1, 1), (3, 3)]);
}

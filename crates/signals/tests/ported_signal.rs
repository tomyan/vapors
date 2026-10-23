//! Ported from `reactive_graph`'s `tests/signal.rs` (leptos-rs/leptos).
//!
//! Not ported, and why:
//! - `create_arc_rw_signal`, `update_arc_rw_signal`, `create_arc_signal`,
//!   `update_arc_signal`: the `Arc` handles exist for multi-threaded use;
//!   this runtime is single-threaded, so `Signal` covers both.
//! - `create_signal`/`update_signal` read-write split: `Signal` is
//!   read-write, so they collapse into the `RwSignal` tests below.
//! - `read()`/`write()` guards and `into_inner`: not part of this API.
//! - `signal_callable_on_stable`: needs the unstable `Fn` traits.

// The originals' short names are kept so the port is easy to check.
#![allow(clippy::many_single_char_names, clippy::items_after_statements)]

use vapors_signals::{Owner, Signal};

#[test]
fn create_rw_signal() {
    // Given
    let owner = Owner::new();
    owner.set();

    // When
    let a = Signal::new(0);

    // Then
    assert_eq!(a.get(), 0);
    assert_eq!(a.get_untracked(), 0);
    assert_eq!(a.with_untracked(|n| n + 1), 1);
    assert_eq!(a.with(|n| n + 1), 1);
}

#[test]
fn update_rw_signal() {
    // Given
    let owner = Owner::new();
    owner.set();
    let a = Signal::new(1);

    // When / Then
    assert_eq!(a.get(), 1);
    a.update(|n| *n += 1);
    assert_eq!(a.get(), 2);
    a.update_untracked(|n| *n += 1);
    assert_eq!(a.get(), 3);
    a.set(4);
    assert_eq!(a.get(), 4);
}

#[test]
fn signal_handle_is_copy_and_shares_the_value() {
    // Given
    let owner = Owner::new();
    owner.set();
    let a = Signal::new(String::from("x"));

    // When
    let b = a;
    b.set(String::from("y"));

    // Then
    assert_eq!(a.get(), "y");
}

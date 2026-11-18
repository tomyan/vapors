//! `ReadSignal`: a read-only view of a signal or a memo that tracks.

use std::cell::Cell;
use std::rc::Rc;

use vapors_signals::{Effect, Memo, ReadSignal, Signal};

/// An effect reading `view`, and the values it saw.
fn watch(view: ReadSignal<i32>) -> Rc<Cell<i32>> {
    let seen = Rc::new(Cell::new(0));
    let out = seen.clone();
    Effect::new(move || out.set(view.get()));
    seen
}

#[test]
fn a_view_of_a_signal_reads_and_tracks() {
    let source = Signal::new(1);
    let seen = watch(source.read_only());
    source.set(5);
    assert_eq!(seen.get(), 5);
    assert_eq!(source.read_only().get_untracked(), 5);
}

#[test]
fn a_view_of_a_memo_reads_and_tracks() {
    let source = Signal::new(1);
    let doubled = Memo::new(move || source.get() * 2);
    let seen = watch(doubled.into());
    source.set(4);
    assert_eq!(seen.get(), 8);
    assert_eq!(ReadSignal::from(doubled).with(|v| *v), 8);
}

#[test]
fn from_a_signal_accepts_either_where_a_view_is_wanted() {
    fn read(view: impl Into<ReadSignal<i32>>) -> i32 {
        view.into().get()
    }
    assert_eq!(read(Signal::new(3)), 3);
    assert_eq!(read(Memo::new(|| 4)), 4);
}

#[test]
fn a_disposed_view_refuses_by_try() {
    let source = Signal::new(1);
    let view = source.read_only();
    source.dispose();
    assert!(view.try_get().is_err());
}

#[test]
fn derived_is_a_view_that_follows_its_inputs() {
    let source = Signal::new(1);
    let view: ReadSignal<i32> = Signal::derived(move || source.get() + 1);
    let seen = watch(view);
    source.set(9);
    assert_eq!(seen.get(), 10);
}

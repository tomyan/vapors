//! `Signal::has_subscribers` and `unowned`: whether anything live reads
//! a signal, and creating a signal no component's owner will dispose.

use vapors_signals::{Effect, Owner, Signal, unowned};

#[test]
fn a_signal_has_subscribers_while_a_reader_is_live() {
    let source = Signal::new(1);
    assert!(!source.has_subscribers(), "nothing reads it yet");
    let owner = Owner::new();
    owner.with(|| {
        Effect::new(move || {
            let _ = source.get();
        })
    });
    assert!(source.has_subscribers(), "an effect reads it");
    owner.dispose();
    assert!(!source.has_subscribers(), "the reader was disposed");
}

#[test]
fn an_untracked_read_is_not_a_subscriber() {
    let source = Signal::new(1);
    let owner = Owner::new();
    owner.with(|| {
        Effect::new(move || {
            let _ = source.get_untracked();
        })
    });
    assert!(!source.has_subscribers());
}

#[test]
fn a_disposed_signal_has_no_subscribers() {
    let source = Signal::new(1);
    source.dispose();
    assert!(!source.has_subscribers());
}

#[test]
fn an_unowned_signal_outlives_the_owner_it_was_made_under() {
    let owner = Owner::new();
    let made = owner.with(|| unowned(|| Signal::new(7)));
    owner.dispose();
    assert_eq!(made.try_get_untracked(), Ok(7));
}

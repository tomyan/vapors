//! A `HoldButton`'s output: the consumer hears each change of the hold
//! through a callback and holds no handle that could write it back.
use std::{cell::Cell, rc::Rc};
use vapors_signals::{Error, Signal};

/// What a `HoldButton` last told its consumer, so unmount can release a
/// hold after the button's own signals are disposed.
pub(crate) struct HoldOutput {
    on_change: Rc<dyn Fn(bool)>,
    told: Cell<bool>,
}

impl HoldOutput {
    pub(crate) fn new(on_change: Rc<dyn Fn(bool)>) -> Rc<Self> {
        Rc::new(Self {
            on_change,
            told: Cell::new(false),
        })
    }

    /// Tell the consumer the hold is now `held`.
    pub(crate) fn send(&self, held: bool) {
        self.told.set(held);
        (self.on_change)(held);
    }

    /// On unmount: a consumer last told `true` is told `false`.
    pub(crate) fn release(&self) {
        if self.told.replace(false) {
            (self.on_change)(false);
        }
    }
}

/// A `HoldButton` `on_change` that writes each change into `consumer`.
///
/// # Panics
/// When `consumer` refuses a write, except the one case below.
#[must_use]
pub fn hold_into(consumer: Signal<bool>) -> Rc<dyn Fn(bool)> {
    Rc::new(move |held| match consumer.try_set(held) {
        Ok(()) => {}
        // RELAXED: disposing an owner disposes every signal beneath it
        // before any cleanup runs, so tearing down a tree that holds
        // both the consumer and a held button reaches the button's
        // release with the consumer already gone, and no one is left to
        // tell. Only a release is let through. Signals running cleanups
        // before disposing nodes would retire this.
        Err(Error::Disposed { .. }) if !held => {}
        Err(error) => panic!("HoldButton could not report a hold of {held}: {error}"),
    })
}

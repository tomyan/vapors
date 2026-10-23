//! `Trigger`: a dependency with no value, for readers that should re-run
//! on demand.

use std::panic::Location;

use crate::signal::Signal;

/// A unit signal: `track` subscribes, `notify` re-runs the subscribers.
#[derive(Debug, Clone, Copy)]
pub struct Trigger {
    signal: Signal<()>,
}

impl Default for Trigger {
    fn default() -> Self {
        Self::new()
    }
}

impl Trigger {
    /// Creates a trigger owned by the current owner.
    #[track_caller]
    #[must_use]
    pub fn new() -> Trigger {
        let created_at = Location::caller();
        let _ = created_at;
        Trigger {
            signal: Signal::named("trigger", ()),
        }
    }

    /// Subscribes the running memo or effect.
    pub fn track(&self) {
        self.signal.with(|()| ());
    }

    /// Re-runs every subscriber.
    pub fn notify(&self) {
        self.signal.set(());
    }
}

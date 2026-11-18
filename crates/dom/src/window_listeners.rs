//! The page-level key listeners of `<vapors:window>`: kept by the tree,
//! heard after an event has bubbled through the target's own chain.

use crate::{Handler, Listener};

/// The events a window listener may take. A tree refuses any other:
/// the compiler only emits these, so another is a hand-written call.
pub const WINDOW_EVENTS: [&str; 2] = ["keydown", "keyup"];

/// What [`Dom::listen_window`](crate::Dom::listen_window) answers, for
/// [`Dom::unlisten_window`](crate::Dom::unlisten_window).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowListener(u64);

/// A tree's window listeners, in the order they were added.
#[derive(Default)]
pub struct WindowListeners {
    issued: u64,
    all: Vec<(WindowListener, String, Listener)>,
}

impl WindowListeners {
    /// Adds `handler` for `event`.
    ///
    /// # Panics
    /// When `event` is not one of [`WINDOW_EVENTS`]: a window listens for
    /// keys only; a pointer listener goes on an element (`onclick`).
    pub fn add(&mut self, event: &str, handler: Handler) -> WindowListener {
        assert!(
            WINDOW_EVENTS.contains(&event),
            "listen_window({event:?}): a window listener takes {WINDOW_EVENTS:?} only; listen on an element for other events"
        );
        self.issued += 1;
        let id = WindowListener(self.issued);
        self.all
            .push((id, event.to_owned(), Listener::from(handler)));
        id
    }

    /// Removes the listener `id` named.
    ///
    /// # Panics
    /// When it is not there: removed already, or from another tree.
    pub fn remove(&mut self, id: WindowListener) {
        let before = self.all.len();
        self.all.retain(|(held, _, _)| *held != id);
        assert!(
            self.all.len() < before,
            "unlisten_window({id:?}): no such window listener; it is removed once, from the tree that issued it"
        );
    }

    /// The listeners for `name`, cloned, as an entry for the end of a
    /// dispatch chain; `None` when there are none.
    #[must_use]
    pub fn heard(&self, name: &str) -> Option<Vec<Listener>> {
        let found: Vec<Listener> = (self.all.iter())
            .filter(|(_, event, _)| event == name)
            .map(|(_, _, listener)| Listener::clone(listener))
            .collect();
        (!found.is_empty()).then_some(found)
    }
}

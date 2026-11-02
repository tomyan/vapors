//! The native IME's state on the handler (E6c): the composition over
//! the focused field, and whether the window has been told about the
//! field. Each of the handler's entry points makes one call here, so
//! what happens before an event is decided, and tested, in one place:
//! a held clear (an IME that cancelled its preedit) is applied before
//! every event that is not the IME's, a redraw and the loop's own
//! messages included, so no stale preedit is drawn or read; and a field
//! set before the window opened gets the IME at the window's first
//! event, which winit sends as it opens it (a resize, a redraw).

use winit::event::WindowEvent;

use crate::ime::Composition;
use crate::ime_window::{self, ImeTarget};
use crate::{Event, TextField};

#[derive(Default)]
pub(crate) struct ImeState {
    composition: Composition,
    /// Whether the window has been told about the field last set.
    told: bool,
}

impl ImeState {
    /// The app set the focused field (or none); the window, if it is
    /// open yet, is told.
    pub(crate) fn mirror(&mut self, window: Option<&impl ImeTarget>, field: Option<&TextField>) {
        ime_window::mirror(window, &mut self.composition, field);
        self.told = window.is_some();
    }

    /// Before `event` on `window`: the IME for a field set before the
    /// window opened, once; then the IME's edit for its own event, or a
    /// held clear for any other.
    pub(crate) fn window_event(
        &mut self,
        window: &impl ImeTarget,
        event: &WindowEvent,
    ) -> Option<Event> {
        if !self.told {
            ime_window::opened(window, &self.composition);
            self.told = true;
        }
        match event {
            WindowEvent::Ime(ime) => self.composition.edit(ime),
            _ => self.composition.flush(),
        }
    }

    /// Before one of the loop's own messages (an input from the page, a
    /// file, a wake): a held clear.
    pub(crate) fn message(&mut self) -> Option<Event> {
        self.composition.flush()
    }
}

#[cfg(test)]
#[path = "ime_state_tests.rs"]
mod tests;

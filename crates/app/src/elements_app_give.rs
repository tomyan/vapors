//! Events into an [`ElementsApp`], alone or under a host. Alone (its
//! `App::event`) the document gets every event, as it has the window to
//! itself. An app that wraps the `ElementsApp` and draws under its
//! document (the racer's track) calls [`ElementsApp::give`] instead: the
//! document gets what is its by `DocumentInput::give`'s rules
//! (`document_input_route.rs`), and the answer says what is left for the
//! app.

use vapors_platform::{Event, Instant, Platform};

use super::ElementsApp;
use crate::document_input::{Given, Route, Unplaced};

/// Whether the document has the window alone or shares it with a host.
#[derive(Clone, Copy)]
pub(super) enum Hosting {
    /// Every event is the document's (`App::event`).
    Alone,
    /// The host draws under the document and takes what it leaves.
    Host,
}

impl ElementsApp {
    /// Delivers `event` for an app that wraps this one and draws under
    /// its document, in place of `App::event`, and says who it was for:
    /// [`Route::Document`] when the document had it alone (a press on its
    /// elements, a key its focused control used), [`Route::Both`] when
    /// the document had it and the app acts on it too (a key the focused
    /// element did not use, a window focus change), [`Route::Host`] when
    /// it was the app's alone and the document did not get it.
    ///
    /// # Errors
    ///
    /// [`Unplaced`], and the document did not get the event, when it is
    /// a pointer event and the document's last update gives no usable
    /// scale: [`Unplaced::NotUpdated`] before the first frame.
    pub fn give(&mut self, event: &Event, platform: &Platform) -> Result<Route, Unplaced> {
        self.deliver(event, platform, Hosting::Host)
    }

    /// An event, dispatched as it arrives (design 02, 2-S1), timed as
    /// the Input phase.
    pub(super) fn deliver(
        &mut self,
        event: &Event,
        platform: &Platform,
        hosting: Hosting,
    ) -> Result<Route, Unplaced> {
        let started = Instant::now();
        self.surfaces.keep_waker(platform);
        let route = self.place(event, platform, hosting);
        self.text_event(event);
        if self.document.borrow_mut().take_changed() {
            platform.request_redraw();
        }
        self.mirror_text_field(platform);
        self.input_spent += started.elapsed();
        route
    }

    /// Hands `event` to the document as `hosting` says, then asks for a
    /// frame if it changed and sets the cursor it asked for.
    fn place(
        &mut self,
        event: &Event,
        platform: &Platform,
        hosting: Hosting,
    ) -> Result<Route, Unplaced> {
        let given = match hosting {
            Hosting::Alone => self.input.handle(&self.document, event, platform)?.into(),
            Hosting::Host => self.input.give(&self.document, event, platform)?,
        };
        if given.changed {
            platform.request_redraw();
        }
        if let Some(cursor) = given.cursor {
            platform.set_cursor(cursor);
        }
        Ok(given.route)
    }
}

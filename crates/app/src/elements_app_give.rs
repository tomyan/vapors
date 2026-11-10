//! Events into an [`ElementsApp`], alone or under a host. Alone (its
//! `App::event`) the document gets every event, as it has the window to
//! itself. An app that wraps the `ElementsApp` and draws under its
//! document (the racer's track) calls [`ElementsApp::give`] instead: the
//! document gets what is its by `DocumentInput::give`'s rules
//! (`document_input_route.rs`), and the answer says what is left for the
//! app.

use std::fmt;

use vapors_platform::{Event, Instant, Platform};

use super::ElementsApp;
use super::text::Text;
use crate::document_input::{Route, Unplaced};

/// Why [`ElementsApp::give`] could not deliver an event.
#[derive(Clone, Debug, PartialEq)]
pub enum GiveError {
    /// A pointer event the document's last update gives no place.
    Unplaced(Unplaced),
    /// The platform's text input edited a text the focused field no
    /// longer holds, or with no field focused: the document refused the
    /// edit for the reason held, and the field is set again for the
    /// platform after the event.
    EditRefused(String),
    /// A paste from the platform's text input arrived with no text
    /// field focused, so nothing was pasted.
    PasteWithoutField,
}

impl From<Unplaced> for GiveError {
    fn from(unplaced: Unplaced) -> Self {
        Self::Unplaced(unplaced)
    }
}

impl fmt::Display for GiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unplaced(unplaced) => unplaced.fmt(f),
            Self::EditRefused(why) => {
                write!(f, "an edit from the page's text input was refused: {why}")
            }
            Self::PasteWithoutField => write!(
                f,
                "a paste from the page's text input arrived with no text field focused, so it was not pasted"
            ),
        }
    }
}

impl std::error::Error for GiveError {}

/// Whether the document has the window alone or shares it with a host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    /// it was the app's alone and the document did not get it. An edit
    /// or paste from the platform's text input that the focused field
    /// took is `Route::Document`.
    ///
    /// Use `give` or `App::event` on one `ElementsApp`, not both: they
    /// keep the routes of held keys and presses apart.
    ///
    /// # Errors
    ///
    /// [`GiveError::Unplaced`], and the document did not get the event,
    /// when it is a pointer event and the document's last update gives no
    /// usable scale: [`Unplaced::NotUpdated`] before the first frame.
    /// [`GiveError::EditRefused`] and [`GiveError::PasteWithoutField`]
    /// when the document refused the platform's text input.
    pub fn give(&mut self, event: &Event, platform: &Platform) -> Result<Route, GiveError> {
        self.deliver(event, platform, Hosting::Host)
    }

    /// An event, dispatched as it arrives (design 02, 2-S1), timed as
    /// the Input phase.
    pub(super) fn deliver(
        &mut self,
        event: &Event,
        platform: &Platform,
        hosting: Hosting,
    ) -> Result<Route, GiveError> {
        let started = Instant::now();
        self.one_hosting(hosting);
        self.surfaces.keep_waker(platform);
        let route = self.place(event, platform, hosting);
        let text = self.text_event(event);
        if self.document.borrow_mut().take_changed() {
            platform.request_redraw();
        }
        self.mirror_text_field(platform);
        self.input_spent += started.elapsed();
        Ok(match text? {
            // the route's rules leave text input to the host; the field took it
            Text::Taken => Route::Document,
            Text::NotText => route?,
        })
    }

    /// Refuses, in a debug build, events through both `App::event` and
    /// `give`: the routes each holds for keys and presses would disagree.
    fn one_hosting(&mut self, hosting: Hosting) {
        let was = self.hosting.replace(hosting);
        debug_assert!(
            was.is_none_or(|was| was == hosting),
            "an ElementsApp got events through both App::event and give; an app uses one (elements_app_give.rs)"
        );
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

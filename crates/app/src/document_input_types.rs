//! What `DocumentInput::give` answers, and why an event could not be
//! placed.

use std::fmt;

use vapors_platform::{Cursor as Shown, Event};

use super::Route;

/// An event given: who it was for, in the end, and what it did to the
/// document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Given {
    /// Who the event was for: the host acts on it when this is
    /// [`Route::Host`] or [`Route::Both`], and only then. A key-down the
    /// document had and did not use (no listener prevented it, no
    /// control took it) is `Both`, as are its repeats and its up.
    pub route: Route,
    /// The document changed: draw a frame.
    pub changed: bool,
    /// The cursor to show, when it is not the one last reported.
    pub cursor: Option<Shown>,
}

/// What handing an event to the document did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Handled {
    pub(crate) changed: bool,
    pub(crate) cursor: Option<Shown>,
    /// A key went down that the document did not use.
    pub(crate) unused_key: bool,
}

/// An event the document had alone (`ElementsApp`'s `App::event`).
impl From<Handled> for Given {
    fn from(handled: Handled) -> Self {
        Given {
            route: Route::Document,
            changed: handled.changed,
            cursor: handled.cursor,
        }
    }
}

/// Why an event could not be placed in the document.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unplaced {
    /// The document has not been updated, so nothing is on screen and
    /// no position can be converted.
    NotUpdated,
    /// The last update's scale is zero, negative or not a number.
    Scale(f32),
    /// A key or typed text arrived after an update with no
    /// `Document::settle_focus` since, while the focused element is not
    /// focusable: a host that drives its own frames forgot the settle.
    FocusNotSettled,
}

impl fmt::Display for Unplaced {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotUpdated => write!(
                f,
                "the document has not been updated, so input has nowhere to land"
            ),
            Self::Scale(scale) => write!(
                f,
                "the document was updated at scale {scale}, which is not a positive number; pass the display's device pixels per CSS pixel in its Viewport"
            ),
            Self::FocusNotSettled => write!(
                f,
                "a key or typed text arrived for a focused element that is no longer focusable, and Document::settle_focus has not run since the last update; call it after each update (ElementsApp does), see docs/tooltips.md \"For apps not using ElementsApp\""
            ),
        }
    }
}

impl std::error::Error for Unplaced {}

/// The scale of `document`'s last update, refused when there is none or
/// it is not a positive number.
pub(super) fn scale_of(document: &vapors_elements::Document) -> Result<f32, Unplaced> {
    let scale = document.painted().ok_or(Unplaced::NotUpdated)?.scale();
    if scale.is_finite() && scale > 0.0 {
        Ok(scale)
    } else {
        Err(Unplaced::Scale(scale))
    }
}

/// Refuses a key or typed text while focus is unsettled
/// ([`Unplaced::FocusNotSettled`]); any other event passes.
pub(super) fn settled(document: &vapors_elements::Document, event: &Event) -> Result<(), Unplaced> {
    let typing = matches!(
        event,
        Event::Key { .. }
            | Event::Character(_)
            | Event::Paste
            | Event::TextEdited { .. }
            | Event::PastedText(_)
    );
    if typing && document.unsettled_focus() {
        return Err(Unplaced::FocusNotSettled);
    }
    Ok(())
}

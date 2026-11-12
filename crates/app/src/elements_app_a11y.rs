//! The Accessibility phase (R6, `docs/plans/r6.md`): with a mirror
//! attached, each frame takes the document's a11y update after its
//! update and keeps it for the consumer, which takes it after the frame
//! (the platform in `drawn`, from PR 5). With none attached the phase
//! does nothing and the document collects nothing. A refused take stops
//! the frame, named; an update left untaken when the next frame takes
//! one is refused too, since replacing it would lose changes silently.
//! The app owns the document's takes while attached
//! (`Document::ax_mirror_attach`): a take elsewhere is refused naming
//! [`ElementsApp::ax_update`].

use vapors_elements::{AxError, AxTaker, AxUpdate};

use super::ElementsApp;

/// The name the document refuses other takers with.
const OWNER: &str = "ElementsApp::ax_update";

/// The attached mirror's state.
#[derive(Default)]
pub(super) struct A11yMirror {
    /// The document's taker while a consumer takes the updates.
    taker: Option<AxTaker>,
    /// The last frame's update, until the consumer takes it.
    pending: Option<AxUpdate>,
}

impl ElementsApp {
    /// Attaches (`true`) or detaches (`false`) the a11y mirror: whether
    /// each frame's Accessibility phase takes the document's update for
    /// [`ElementsApp::ax_update`]. Detaching drops an untaken update and
    /// resets the document's takes, so a mirror attached later starts
    /// with every node, as a first take does. Attaching or detaching
    /// twice changes nothing.
    ///
    /// # Errors
    /// [`AxError::Owned`]: another consumer (a second app on the same
    /// document) owns the takes.
    pub fn mirror_a11y(&mut self, on: bool) -> Result<(), AxError> {
        match (on, self.a11y.taker.take()) {
            (true, None) => {
                let taker = self.document.borrow_mut().ax_mirror_attach(OWNER)?;
                self.a11y.taker = Some(taker);
            }
            (true, Some(taker)) => self.a11y.taker = Some(taker),
            (false, Some(taker)) => {
                self.document.borrow_mut().ax_mirror_detach(taker)?;
                self.a11y.pending = None;
            }
            (false, None) => {}
        }
        Ok(())
    }

    /// The last frame's a11y update, once; `None` when nothing changed,
    /// no mirror is attached, or it was taken already.
    #[must_use]
    pub fn ax_update(&mut self) -> Option<AxUpdate> {
        self.a11y.pending.take()
    }

    /// The Accessibility phase's work: the document's take, kept.
    pub(super) fn take_a11y(&mut self) -> Result<(), String> {
        let Some(taker) = &self.a11y.taker else {
            return Ok(());
        };
        if self.a11y.pending.is_some() {
            return Err(
                "the last frame's a11y update was not taken, so this frame's would lose it: \
                 call ElementsApp::ax_update after every frame"
                    .to_owned(),
            );
        }
        let update = self.document.borrow_mut().take_ax_update_by(taker);
        self.a11y.pending = update.map_err(|refused| refused.to_string())?;
        Ok(())
    }
}

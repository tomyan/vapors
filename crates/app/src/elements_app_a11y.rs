//! The Accessibility phase (R6, `docs/plans/r6.md`): with a mirror
//! attached, each frame takes the document's a11y update after its
//! update and keeps it for the consumer, which takes it after the frame
//! (the platform in `drawn`, from PR 5). With none attached the phase
//! does nothing and the document collects nothing. A refused take stops
//! the frame, named; an update left untaken when the next frame takes
//! one is refused too, since replacing it would lose changes silently.

use vapors_elements::AxUpdate;

use super::ElementsApp;

/// The attached mirror's state.
#[derive(Default)]
pub(super) struct A11yMirror {
    /// Whether a consumer takes the updates.
    on: bool,
    /// The last frame's update, until the consumer takes it.
    pending: Option<AxUpdate>,
}

impl ElementsApp {
    /// Attaches (`true`) or detaches (`false`) the a11y mirror: whether
    /// each frame's Accessibility phase takes the document's update for
    /// [`ElementsApp::ax_update`]. Detaching drops an untaken update and
    /// resets the document's takes, so a mirror attached later starts
    /// with every node, as a first take does.
    pub fn mirror_a11y(&mut self, on: bool) {
        if !on {
            self.document.borrow_mut().ax_mirror_detach();
            self.a11y.pending = None;
        }
        self.a11y.on = on;
    }

    /// The last frame's a11y update, once; `None` when nothing changed,
    /// no mirror is attached, or it was taken already.
    #[must_use]
    pub fn ax_update(&mut self) -> Option<AxUpdate> {
        self.a11y.pending.take()
    }

    /// The Accessibility phase's work: the document's take, kept.
    pub(super) fn take_a11y(&mut self) -> Result<(), String> {
        if !self.a11y.on {
            return Ok(());
        }
        if self.a11y.pending.is_some() {
            return Err(
                "the last frame's a11y update was not taken, so this frame's would lose it: \
                 call ElementsApp::ax_update after every frame"
                    .to_owned(),
            );
        }
        let taken = self.document.borrow_mut().take_ax_update();
        self.a11y.pending = taken.map_err(|refused| refused.to_string())?;
        Ok(())
    }
}

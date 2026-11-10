//! Why the app asked for the next frame, kept for a failing check to
//! print (`ElementsApp::last_decision`).

use vapors_platform::{Instant, Next};

use super::ElementsApp;
use crate::painter::{ScenePainter, SurfaceState};

/// What `next_after_frame` found at the end of a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameDecision {
    /// The document changed since it was last asked.
    pub document_changed: bool,
    /// A surface was invalidated since the frame began.
    pub surface_invalidated: bool,
    /// An active surface is stale or animating.
    pub surfaces_want: bool,
}

impl FrameDecision {
    /// Whether any reason holds: the app draws again.
    #[must_use]
    pub fn wants_frame(self) -> bool {
        self.document_changed || self.surface_invalidated || self.surfaces_want
    }

    /// What the loop does: draw again at once when a reason holds, else
    /// wait, until `scope_deadline` when a surface's GPU error scope is
    /// unresolved, so that a frame then refuses it if no result came.
    #[must_use]
    pub fn next(self, scope_deadline: Option<Instant>) -> Next {
        match (self.wants_frame(), scope_deadline) {
            (true, _) => Next::Continue,
            (false, Some(at)) => Next::WaitUntil(at),
            (false, None) => Next::Wait,
        }
    }
}

impl ElementsApp {
    /// What to do after a frame: draw again at once if the document
    /// changed since it was last asked (by something outside an event,
    /// or by the update itself), or a surface was invalidated or wants
    /// a frame (`ScenePainter::wants_frame`), else wait for the next
    /// change or a GPU error scope's deadline (`FrameDecision::next`).
    pub fn next_after_frame(&mut self) -> Next {
        self.decision = FrameDecision {
            document_changed: self.document.borrow_mut().take_changed(),
            surface_invalidated: self.surfaces.any_stale(),
            surfaces_want: self.painter.as_ref().is_some_and(ScenePainter::wants_frame),
        };
        let scope_deadline = self.painter.as_ref().and_then(ScenePainter::scope_deadline);
        self.decision.next(scope_deadline)
    }

    /// Brings the painter's animating flags up to date, after the
    /// frame's phases and before `next_after_frame` reads them: a
    /// `stop_animation` made after the paint ends this frame's request
    /// for the next.
    pub(super) fn sync_animating(&mut self) -> Result<(), String> {
        let Some(painter) = self.painter.as_mut() else {
            return Ok(());
        };
        self.surfaces
            .sync_animating(painter)
            .map_err(|error| format!("the surfaces could not be synced: {error}"))
    }

    /// The reasons at the end of the last frame; all false before one.
    #[must_use]
    pub fn last_decision(&self) -> FrameDecision {
        self.decision
    }

    /// Each surface's state on the painter; empty before the first frame.
    #[must_use]
    pub fn surface_states(&self) -> Vec<SurfaceState> {
        self.painter
            .as_ref()
            .map_or_else(Vec::new, ScenePainter::surface_states)
    }
}

#[cfg(test)]
#[path = "elements_app_decision_tests.rs"]
mod tests;

//! Why the app asked for the next frame, kept for a failing check to
//! print (`ElementsApp::last_decision`).

use vapors_platform::{Instant, Next};

use super::ElementsApp;
use crate::painter::{ScenePainter, SurfaceState};

/// What `next_after_frame` found at the end of a frame.
// each bool is an independent reason, printed by a failing check; an
// enum or bitflags would not let one frame hold several
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameDecision {
    /// The document changed since it was last asked.
    pub document_changed: bool,
    /// A surface was invalidated since the frame began.
    pub surface_invalidated: bool,
    /// An active surface is stale, animating or waiting on a GPU scope.
    pub surfaces_want: bool,
    /// When the armed tooltip shows (`DocumentInput::tooltip_deadline`).
    pub tooltip_deadline: Option<Instant>,
    /// The tooltip armed or shown changed since an event last asked
    /// (`DocumentInput::take_tooltip_changed`): this frame's update
    /// showed, re-armed or hid it, and its draw has that, so it asks
    /// for no frame. Taking it here keeps the next event from asking
    /// for a frame for it again.
    pub tooltip_changed: bool,
}

impl FrameDecision {
    /// Whether any reason holds: the app draws again at once.
    #[must_use]
    pub fn wants_frame(self) -> bool {
        self.document_changed || self.surface_invalidated || self.surfaces_want
    }

    /// The next frame: at once when a reason holds, else at the
    /// tooltip's deadline, else on the next change.
    #[must_use]
    pub fn next(self) -> Next {
        match (self.wants_frame(), self.tooltip_deadline) {
            (true, _) => Next::Continue,
            (false, Some(deadline)) => Next::WaitUntil(deadline),
            (false, None) => Next::Wait,
        }
    }
}

impl ElementsApp {
    /// What to do after a frame: draw again at once if the document
    /// changed since it was last asked (by something outside an event,
    /// or by the update itself), or a surface was invalidated or wants
    /// a frame (`ScenePainter::wants_frame`), else wait until an armed
    /// tooltip's deadline, asked again every frame until it shows, or
    /// for the next change.
    pub fn next_after_frame(&mut self) -> Next {
        self.decision = FrameDecision {
            document_changed: self.document.borrow_mut().take_changed(),
            surface_invalidated: self.surfaces.any_stale(),
            surfaces_want: self.painter.as_ref().is_some_and(ScenePainter::wants_frame),
            tooltip_deadline: self.input.tooltip_deadline(),
            tooltip_changed: self.input.take_tooltip_changed(),
        };
        self.decision.next()
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
mod tests {
    use std::time::Duration;

    use vapors_platform::{Instant, Next};

    use super::FrameDecision;

    #[test]
    fn an_armed_tooltip_waits_until_its_deadline_and_none_waits_for_a_change() {
        // Given
        let deadline = Instant::now() + Duration::from_millis(500);
        let armed = FrameDecision {
            tooltip_deadline: Some(deadline),
            ..FrameDecision::default()
        };
        let changed = FrameDecision {
            document_changed: true,
            ..armed
        };

        // Then
        assert_eq!(armed.next(), Next::WaitUntil(deadline));
        assert_eq!(changed.next(), Next::Continue);
        assert_eq!(FrameDecision::default().next(), Next::Wait);
    }
}

//! What a redraw does after the surface gave no texture, and which
//! window events end a wait for a hidden window.

use std::time::Duration;

use winit::event::WindowEvent;

use crate::surface_texture::Miss;

/// How long a hidden window waits before one more try, for a platform
/// that reports no event when the window is shown again. Unmeasured:
/// one try per half second costs about two `get_current_texture` calls
/// a second, and shows the window again within half a second. An
/// `Occluded(false)`, `Resized` or `Focused(true)` event retries at once.
pub const RETRY_AFTER: Duration = Duration::from_millis(500);

/// What the handler does after a redraw with no texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissAction {
    /// The surface failed: it is reconfigured, a redraw is already
    /// requested, and this many redraws in a row have failed.
    Count(u32),
    /// Too many redraws in a row failed: stop the loop, naming the last
    /// outcome.
    Fail(&'static str),
    /// The window is hidden: request no redraw, retry after
    /// [`RETRY_AFTER`] or on an event that [`shows_window`] accepts.
    Wait,
}

/// The action for `miss`, when `missed` redraws before it failed in a
/// row and `limit` in a row stop the loop.
#[must_use]
pub fn after_miss(miss: Miss, missed: u32, limit: u32) -> MissAction {
    match miss {
        Miss::Waiting(_) => MissAction::Wait,
        Miss::Failed(outcome) if missed.saturating_add(1) >= limit => MissAction::Fail(outcome),
        Miss::Failed(_) => MissAction::Count(missed + 1),
    }
}

/// True for an event after which a hidden window may be shown again.
#[must_use]
pub fn shows_window(event: &WindowEvent) -> bool {
    matches!(
        event,
        WindowEvent::Occluded(false) | WindowEvent::Resized(_) | WindowEvent::Focused(true)
    )
}

#[cfg(test)]
#[path = "hidden_window_tests.rs"]
mod tests;

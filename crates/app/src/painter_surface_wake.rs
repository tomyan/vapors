//! When the painter's surfaces want the next frame: at once while one
//! is stale or animating, or at an unresolved GPU error scope's
//! deadline, or when the waker it was given is woken by a scope's
//! result (docs/gpu-surface.md, Animation).

use std::task::Waker;

use vapors_platform::Instant;

use super::super::ScenePainter;
use super::Mounted;

impl ScenePainter {
    /// True when an active surface (one the last frame painted) is
    /// invalidated or animating: the app should draw another frame.
    #[must_use]
    pub fn wants_frame(&self) -> bool {
        self.surfaces.mounted.values().any(Mounted::wants_frame)
    }

    /// Sets the waker a surface's GPU error scope wakes when its result
    /// arrives after the frame (on the web): the owner's loop then draws
    /// a frame that checks it. Without one, the result is seen on the
    /// next frame drawn for another reason, or at [`Self::scope_deadline`].
    /// Scopes polled before it was set wake it now, for a frame that
    /// polls them with it.
    pub fn wake_with(&mut self, waker: Waker) {
        if self.scope_deadline().is_some() {
            waker.wake_by_ref();
        }
        self.surfaces.waker = Some(waker);
    }

    /// The earliest time an unresolved GPU error scope is refused, if a
    /// surface has one: the owner's loop draws a frame no later than this
    /// (`Next::WaitUntil`), which refuses the surface if it is still
    /// unresolved. A pending scope does not make [`Self::wants_frame`] true.
    #[must_use]
    pub fn scope_deadline(&self) -> Option<Instant> {
        let mounted = self.surfaces.mounted.values();
        mounted.filter_map(Mounted::deadline).min()
    }
}

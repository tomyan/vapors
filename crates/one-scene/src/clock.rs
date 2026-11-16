//! The page's clock: its time signal from the frames' own clock, so the
//! frame clock gives a deterministic run, and its frame counter. With a
//! limit (the web page's `ticks=N`), the clock stops after N frames and
//! the surface stops animating, so a headless page goes idle for its
//! screenshot, as the demos' `ticks` does.

use std::time::Duration;

use vapors_app::SurfaceHandle;
use vapors_platform::Instant;
use vapors_signals::Signal;

pub(crate) struct Clock {
    pub(crate) time: Signal<Duration>,
    pub(crate) frames: Signal<u64>,
    started: Option<Instant>,
    limit: Option<u64>,
    surface: SurfaceHandle,
}

impl Clock {
    pub(crate) fn new(surface: SurfaceHandle, limit: Option<u64>) -> Self {
        Self {
            time: Signal::new(Duration::ZERO),
            frames: Signal::new(0),
            started: None,
            limit,
            surface,
        }
    }

    /// Advances to the frame drawn at `now`, unless the limit is reached;
    /// stops the surface's animation at the limit.
    ///
    /// # Errors
    /// The surface refused to stop animating.
    pub(crate) fn tick(&mut self, now: Instant) -> Result<(), String> {
        let ticked = self.frames.get_untracked();
        if self.limit.is_some_and(|limit| ticked >= limit) {
            return Ok(());
        }
        let started = *self.started.get_or_insert(now);
        self.time.set(now.duration_since(started));
        self.frames.set(ticked + 1);
        if self.limit == Some(ticked + 1) {
            self.surface.stop_animation().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

//! The time an event is handled at (#440): the platform's stamp for it
//! (`Platform::now`), never earlier than the last frame's time, which
//! the thumbs' fade and a fling have already been moved to. An earlier
//! stamp (a queued event handled after a frame) is clamped to the frame
//! and counted, never left to move a fade back unseen.

use vapors_platform::Instant;

/// The last frame's time, and how many event times were clamped to it.
#[derive(Debug, Default)]
pub(crate) struct EventClock {
    last_frame: Option<Instant>,
    clamped: usize,
}

impl EventClock {
    /// A frame ran at `now`.
    pub(crate) fn framed(&mut self, now: Instant) {
        self.last_frame = Some(self.last_frame.map_or(now, |last| last.max(now)));
    }

    /// The time to handle an event the platform stamped `stamped` at.
    pub(crate) fn at(&mut self, stamped: Instant) -> Instant {
        match self.last_frame {
            Some(frame) if stamped < frame => {
                self.clamped += 1;
                frame
            }
            _ => stamped,
        }
    }

    /// How many event times were earlier than the last frame's.
    pub(crate) fn clamped(&self) -> usize {
        self.clamped
    }
}

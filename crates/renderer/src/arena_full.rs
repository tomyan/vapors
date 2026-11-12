//! A frame whose draws need a larger buffer than the device allows.

use std::fmt;

/// A pipeline's [`FrameArena`](crate::FrameArena) needed a buffer larger
/// than the device's `max_buffer_size` to hold one frame's draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArenaFull {
    /// The pipeline's label.
    pub pipeline: &'static str,
    /// The buffer size, in bytes, the frame's draws needed.
    pub size: u64,
    /// The device's `max_buffer_size`.
    pub limit: u64,
}

impl fmt::Display for ArenaFull {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: one frame's draws need a {}-byte buffer, over the device's max_buffer_size of {} bytes; draw less in one frame",
            self.pipeline, self.size, self.limit
        )
    }
}

impl std::error::Error for ArenaFull {}

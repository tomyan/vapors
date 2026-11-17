//! A frame whose draws, or the bytes kept between frames, need a larger
//! buffer than the device allows.

use std::fmt;

/// A pipeline's [`FrameArena`](crate::FrameArena) needed a buffer larger
/// than the device's `max_buffer_size` to hold one frame's draws, or its
/// [`Retained`](crate::Retained) buffer to hold the bytes kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArenaFull {
    /// The pipeline's label.
    pub pipeline: &'static str,
    /// The buffer size, in bytes, the frame's draws needed.
    pub size: u64,
    /// The device's limit that applied: `max_buffer_size`, or for the
    /// transform table the lower of it and
    /// `max_storage_buffer_binding_size` ([`Holding::TransformEntries`]).
    pub limit: u64,
    /// What the buffer holds.
    pub holding: Holding,
}

/// What a refused buffer holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holding {
    /// One frame's draws.
    OneFrame,
    /// Bytes kept between frames, until released.
    Kept,
    /// The transform table's entries (`transform_table.rs`): one per frame
    /// drawn and a range per lane batch drawn, kept while drawn; `limit`
    /// names the device limit that applied.
    TransformEntries {
        /// `max_buffer_size` or `max_storage_buffer_binding_size`.
        limit: &'static str,
    },
}

impl fmt::Display for ArenaFull {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (what, limit, fix) = match self.holding {
            Holding::OneFrame => (
                "one frame's draws need",
                "max_buffer_size",
                "draw less in one frame",
            ),
            Holding::Kept => (
                "the bytes kept between frames need",
                "max_buffer_size",
                "release kept ranges or keep fewer",
            ),
            Holding::TransformEntries { limit } => (
                "the transform table's entries need",
                limit,
                "draw fewer scroll frames and lane batches in view, each of which holds entries \
                 while it is drawn",
            ),
        };
        write!(
            f,
            "{}: {what} a {}-byte buffer, over the device's {limit} of {} bytes; {fix}",
            self.pipeline, self.size, self.limit
        )
    }
}

impl std::error::Error for ArenaFull {}

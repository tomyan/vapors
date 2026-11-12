//! Where one frame's timestamps are in their trip back from the GPU:
//! free, recorded into, mapping for reading, or already known to be
//! missing. Kept apart from the buffers so each step can be tested
//! without a GPU.

use std::sync::mpsc::{Receiver, TryRecvError};

use crate::gpu_time::{FrameReading, GpuMissing};

/// The map's outcome, sent by its callback.
pub(crate) type MapOutcome = Result<(), wgpu::BufferAsyncError>;

/// One slot of the ring.
#[derive(Debug, Default)]
pub(crate) enum SlotState {
    /// Nothing in it.
    #[default]
    Free,
    /// A frame's encoder is writing timestamps into it.
    Recording { frame: u64 },
    /// Resolved and submitted; the read-back buffer is being mapped.
    Mapping {
        frame: u64,
        pairs: u32,
        outcome: Receiver<MapOutcome>,
    },
    /// Known without reading: missing, or no pairs written.
    Done(FrameReading),
}

/// What a slot holds once looked at.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Finished {
    /// Nothing finished: free, recording or still mapping.
    Nothing,
    /// The read-back buffer is mapped: read `pairs` pairs, then unmap it.
    Mapped { frame: u64, pairs: u32 },
    /// The frame's reading, with nothing to read.
    Reading(FrameReading),
}

impl SlotState {
    /// The slot's finished frame, leaving it free; a mapping not yet
    /// done stays.
    pub(crate) fn take_finished(&mut self) -> Finished {
        let outcome = match self {
            Self::Mapping { outcome, .. } => outcome.try_recv(),
            Self::Done(_) => Err(TryRecvError::Disconnected),
            Self::Free | Self::Recording { .. } => return Finished::Nothing,
        };
        if outcome == Err(TryRecvError::Empty) {
            return Finished::Nothing;
        }
        match (std::mem::take(self), outcome) {
            (Self::Done(reading), _) => Finished::Reading(reading),
            (Self::Mapping { frame, pairs, .. }, Ok(Ok(()))) => Finished::Mapped { frame, pairs },
            (Self::Mapping { frame, pairs, .. }, failed) => {
                Finished::Reading(missing(frame, pairs, map_failure(failed)))
            }
            _ => unreachable!("only Mapping and Done reach here"),
        }
    }

    /// Frees the slot for a new frame, `frames` frames after the one in
    /// it: a mapping not done by now is missing.
    ///
    /// # Panics
    /// If the slot is still being recorded into: as many encoders as the
    /// ring has slots are open at once, which no frame loop does.
    pub(crate) fn reclaim(&mut self, frames: usize) -> Option<FrameReading> {
        match std::mem::take(self) {
            Self::Free => None,
            Self::Recording { frame } => panic!(
                "frame {frame}'s encoder is still open {frames} frames later; submit or drop \
                 each frame's FrameEncoder before beginning the next"
            ),
            Self::Mapping { frame, pairs, .. } => {
                Some(missing(frame, pairs, GpuMissing::Deadline { frames }))
            }
            Self::Done(reading) => Some(reading),
        }
    }
}

/// A reading of `frame` with no time.
pub(crate) fn missing(frame: u64, pairs: u32, why: GpuMissing) -> FrameReading {
    FrameReading {
        frame,
        pairs,
        time: Err(why),
    }
}

fn map_failure(outcome: Result<MapOutcome, TryRecvError>) -> GpuMissing {
    match outcome {
        Ok(Err(error)) => GpuMissing::MapFailed(error.to_string()),
        _ => GpuMissing::MapFailed("the map's callback was dropped unanswered".to_owned()),
    }
}

#[cfg(test)]
#[path = "gpu_timer_slot_tests.rs"]
mod tests;

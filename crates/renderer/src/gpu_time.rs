//! A frame's GPU time as timestamp queries measure it: written at the
//! start and end of each of its render passes, read back by a later
//! frame. Measured, missing with the reason, or not measurable on this
//! device; never estimated.

use std::fmt;
use std::time::Duration;

/// What is known of the GPU's time for frames drawn so far.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuTime {
    /// The device has no `TIMESTAMP_QUERY`, so no frame is timed.
    Unsupported,
    /// No timed frame has been read back yet.
    Pending,
    /// The latest timed frame read back.
    Read(GpuReading),
}

/// One frame's timestamps, read back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuReading {
    /// The frame's index, as the app was given it (`Frame::index`).
    pub frame: u64,
    /// Timestamp pairs the frame wrote: one per render pass of its
    /// encoder, one per `gpu-surface` render timed.
    pub pairs: u32,
    /// The sum of the pairs' spans, or why there is none. The GPU's
    /// work outside render passes (buffer writes, copies) is not in it.
    pub time: Result<Duration, GpuMissing>,
}

/// Why a timed frame has no GPU time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuMissing {
    /// The frame's encoder was dropped without being submitted.
    NotSubmitted,
    /// The frame began more passes than its query set holds pairs for.
    OverCapacity {
        /// Pairs the frame needed.
        pairs: u32,
        /// Pairs one frame's query set holds.
        capacity: u32,
    },
    /// The read-back buffer could not be mapped.
    MapFailed(String),
    /// The read-back was not mapped by the time this many later frames
    /// had begun, when its slot was needed again.
    Deadline {
        /// Frames begun after the timed one.
        frames: usize,
    },
    /// A pass's end timestamp was before its start.
    Backwards {
        /// The start tick.
        start: u64,
        /// The end tick.
        end: u64,
    },
}

impl fmt::Display for GpuMissing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSubmitted => write!(f, "the frame's encoder was dropped unsubmitted"),
            Self::OverCapacity { pairs, capacity } => write!(
                f,
                "the frame needed {pairs} timestamp pairs and its query set holds {capacity}; \
                 make the GpuTimer with_capacity of at least {pairs}"
            ),
            Self::MapFailed(error) => write!(f, "mapping the timestamps failed: {error}"),
            Self::Deadline { frames } => write!(
                f,
                "the timestamps were not read back within {frames} frames: the GPU is that \
                 far behind, or the device is not being polled"
            ),
            Self::Backwards { start, end } => {
                write!(f, "a pass ended at tick {end}, before its start {start}")
            }
        }
    }
}

/// The sum of the spans of `ticks`' pairs (start, end), each tick
/// `period` nanoseconds long.
///
/// # Errors
/// [`GpuMissing::Backwards`] for a pair whose end is before its start.
pub(crate) fn sum_pairs(ticks: &[u64], period: f64) -> Result<Duration, GpuMissing> {
    let mut sum = 0u64;
    for &[start, end] in ticks.as_chunks::<2>().0 {
        sum += end
            .checked_sub(start)
            .ok_or(GpuMissing::Backwards { start, end })?;
    }
    Ok(nanos(sum, period))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)] // ticks of a frame are far below 2^52, and the product is positive
fn nanos(ticks: u64, period: f64) -> Duration {
    Duration::from_nanos((ticks as f64 * period) as u64)
}

#[cfg(test)]
#[path = "gpu_time_tests.rs"]
mod tests;

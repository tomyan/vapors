//! What an [`ElementsApp`]'s frames took on the GPU: timestamp pairs
//! around each render pass and each `gpu-surface` render, read back by
//! a later frame ([`GpuTimer`]).

use vapors_platform::Frame;
use vapors_renderer::{FrameEncoder, GpuTime, GpuTimer};

use super::ElementsApp;

/// What is known of the app's frame times, beside
/// [`FrameCounts`](crate::FrameCounts).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct FrameTiming {
    /// The GPU's time for the latest frame read back: frame `n`'s
    /// reading arrives at the start of a frame after `n`, once the GPU
    /// has finished it. [`GpuTime::Pending`] before the first paint.
    pub gpu: GpuTime,
}

/// The app's GPU timer, made at the first paint, when the device is known.
#[derive(Default)]
pub(super) enum AppTimer {
    /// No frame has painted yet.
    #[default]
    Unmade,
    /// The device has `TIMESTAMP_QUERY`.
    Timing(GpuTimer),
    /// The device has no `TIMESTAMP_QUERY`.
    Unsupported,
}

impl AppTimer {
    /// The frame's encoder: timed when the device can be.
    pub(super) fn encoder(&mut self, frame: &Frame<'_>) -> FrameEncoder {
        if matches!(self, Self::Unmade) {
            *self =
                GpuTimer::new(frame.device, frame.queue).map_or(Self::Unsupported, Self::Timing);
        }
        match self {
            Self::Timing(timer) => FrameEncoder::timed(frame.device, timer.frame(frame.index)),
            Self::Unmade | Self::Unsupported => FrameEncoder::new(frame.device),
        }
    }

    fn gpu(&self) -> GpuTime {
        match self {
            Self::Unmade => GpuTime::Pending,
            Self::Timing(timer) => timer.time(),
            Self::Unsupported => GpuTime::Unsupported,
        }
    }
}

impl ElementsApp {
    /// The app's frame times so far: see [`FrameTiming`].
    #[must_use]
    pub fn frame_timing(&self) -> FrameTiming {
        FrameTiming {
            gpu: self.timer.gpu(),
        }
    }
}

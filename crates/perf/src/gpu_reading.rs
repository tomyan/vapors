//! A timed frame's GPU time from the app's `FrameTiming`: the sum of
//! the timestamp pairs around its render passes and `gpu-surface`
//! renders, read back at the start of the frame after it.

use std::time::Duration;

use vapors_app::GpuTime;

/// Frame `frame`'s GPU time from `time`, read after the frame after it;
/// `None` when the device has no timestamp queries.
///
/// # Errors
/// No reading, a reading of another frame, or one that says why it has
/// no time: the harness refuses to report a frame it could not time.
pub fn frame_gpu_time(time: GpuTime, frame: usize) -> Result<Option<Duration>, String> {
    let reading = match time {
        GpuTime::Unsupported => return Ok(None),
        GpuTime::NotRequested => {
            return Err(format!(
                "frame {frame}: the workload's app does not time its frames; call \
                 time_frames(true) on it"
            ));
        }
        GpuTime::Pending => return Err(format!("frame {frame}: no GPU time was read back")),
        GpuTime::Read(reading) => reading,
    };
    if usize::try_from(reading.frame) != Ok(frame) {
        return Err(format!(
            "frame {frame}: the GPU time read back is frame {}'s",
            reading.frame
        ));
    }
    reading
        .time
        .map(Some)
        .map_err(|missing| format!("frame {frame} has no GPU time: {missing}"))
}

#[cfg(test)]
#[path = "gpu_reading_tests.rs"]
mod tests;

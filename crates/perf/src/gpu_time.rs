//! The GPU's time across a frame, from timestamp queries: one written
//! in its own submit before the frame and one after it. The span covers
//! the frame's GPU work and any time the GPU waited between the frame's
//! submits (the painter submits each run of items as it encodes it), so
//! it bounds the frame's GPU time from above. Read back once, after
//! the last frame.

use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;

/// The features the clock needs: timestamps written between submits.
pub const FEATURES: wgpu::Features =
    wgpu::Features::TIMESTAMP_QUERY.union(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS);

/// How long reading the timestamps back may take. The read is of a few
/// kilobytes after the GPU has finished every frame; ten seconds is far
/// beyond it on lavapipe too.
const READ_DEADLINE: Duration = Duration::from_secs(10);

/// Timestamps for up to `capacity` frames.
pub struct GpuClock {
    queries: wgpu::QuerySet,
    written: u32,
    capacity: u32,
}

impl GpuClock {
    /// A clock for `frames` frames; `None` when `device` lacks [`FEATURES`].
    #[must_use]
    pub fn new(device: &wgpu::Device, frames: u32) -> Option<Self> {
        if !device.features().contains(FEATURES) {
            return None;
        }
        let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("frame timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: frames * 2,
        });
        Some(Self {
            queries,
            written: 0,
            capacity: frames * 2,
        })
    }

    /// Writes the next timestamp, in a submit of its own.
    ///
    /// # Panics
    /// When every timestamp has been written: the caller asked for too few.
    pub fn mark(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        assert!(self.written < self.capacity, "the GPU clock is full");
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.write_timestamp(&self.queries, self.written);
        queue.submit([encoder.finish()]);
        self.written += 1;
    }

    /// Each frame's span, from its pair of timestamps.
    ///
    /// # Errors
    /// The read failed or passed its deadline.
    pub fn spans(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<Duration>, String> {
        let ticks = self.read(device, queue)?;
        let period = f64::from(queue.get_timestamp_period());
        ticks
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| span(pair, period))
            .collect()
    }

    fn read(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Vec<u64>, String> {
        let readable = self.resolve(device, queue);
        map(device, &readable)?;
        let bytes = readable
            .slice(..)
            .get_mapped_range()
            .map_err(|error| format!("the timestamps could not be read back: {error}"))?;
        Ok(bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| u64::from_le_bytes(*b))
            .collect())
    }

    /// Submits the copy of the written timestamps into a mappable buffer.
    fn resolve(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Buffer {
        let size = u64::from(self.written) * 8;
        let usage = wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC;
        let resolved = buffer(device, size, usage);
        let usage = wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST;
        let readable = buffer(device, size, usage);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.resolve_query_set(&self.queries, 0..self.written, &resolved, 0);
        encoder.copy_buffer_to_buffer(&resolved, 0, &readable, 0, size);
        queue.submit([encoder.finish()]);
        readable
    }
}

/// Maps `readable` for reading, waiting at most [`READ_DEADLINE`].
fn map(device: &wgpu::Device, readable: &wgpu::Buffer) -> Result<(), String> {
    let (sender, receiver) = mpsc::channel();
    readable
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    wait(device, READ_DEADLINE)?;
    mapped(receiver.try_recv())
}

/// The map's outcome: mapped, refused, or not reported after the wait.
fn mapped(outcome: Result<Result<(), wgpu::BufferAsyncError>, TryRecvError>) -> Result<(), String> {
    match outcome {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(format!(
            "mapping the timestamps for reading failed: {error}"
        )),
        Err(_) => Err(format!(
            "mapping the timestamps had not finished after the GPU wait of {READ_DEADLINE:?}"
        )),
    }
}

/// Waits for the GPU to finish what was submitted, for at most `deadline`.
///
/// # Errors
/// The wait failed or passed `deadline`, naming it.
pub fn wait(device: &wgpu::Device, deadline: Duration) -> Result<(), String> {
    let poll = wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(deadline),
    };
    device
        .poll(poll)
        .map(|_| ())
        .map_err(|error| format!("waiting {deadline:?} for the GPU failed: {error}"))
}

fn buffer(device: &wgpu::Device, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage,
        mapped_at_creation: false,
    })
}

/// The time between a pair of ticks `period` nanoseconds long.
///
/// # Errors
/// The second tick is before the first.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn span(pair: &[u64; 2], period: f64) -> Result<Duration, String> {
    let ticks = pair[1].checked_sub(pair[0]).ok_or_else(|| {
        format!(
            "a frame's end timestamp {} is before its start {}",
            pair[1], pair[0]
        )
    })?;
    Ok(Duration::from_nanos((ticks as f64 * period) as u64))
}

#[cfg(test)]
#[path = "gpu_time_tests.rs"]
mod tests;

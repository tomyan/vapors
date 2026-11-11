//! One command encoder per frame: every pipeline records its passes
//! into it through a [`Target`](crate::Target), and the frame submits
//! it once. On ANV one submit costs about 70 µs
//! (docs/perf/baseline-2026-11-10.md, finding 1), so a submit per draw
//! made draw time linear in draw calls.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};

/// Each encoder's number, so a pipeline's [`FrameArena`](crate::FrameArena)
/// knows when a new frame starts.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// The frame's command encoder, shared through `&self` by every draw.
///
/// A frame of the app's painter submits once, plus once more for each
/// of: a glyph atlas repack (see [`FrameEncoder::flush`]), and each
/// `gpu-surface` that renders, whose encoder is submitted inside the
/// surface's GPU error scope so its validation errors are its own.
///
/// The pipelines write each draw's data to its own place in their
/// buffers and reuse those places from the next encoder on: submit (or
/// drop) one encoder before drawing with the next through the same
/// pipeline. A pipeline given an older encoder after a newer one panics.
pub struct FrameEncoder {
    device: wgpu::Device,
    encoder: RefCell<wgpu::CommandEncoder>,
    id: u64,
}

impl FrameEncoder {
    /// An empty encoder on `device`.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            device: device.clone(),
            encoder: RefCell::new(encoder(device)),
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// Runs `record` with the encoder.
    ///
    /// # Panics
    ///
    /// If `record` itself calls `record` on this encoder.
    pub fn record<R>(&self, record: impl FnOnce(&mut wgpu::CommandEncoder) -> R) -> R {
        record(&mut self.encoder.borrow_mut())
    }

    /// Submits what is recorded so far and goes on recording into a new
    /// encoder: for a change to a resource that recorded draws read and
    /// later draws must see changed, such as a repacked glyph atlas page.
    pub fn flush(&self, queue: &wgpu::Queue) {
        let done = self.encoder.replace(encoder(&self.device));
        queue.submit([done.finish()]);
    }

    /// Submits everything recorded on `queue`.
    pub fn submit(self, queue: &wgpu::Queue) {
        queue.submit([self.encoder.into_inner().finish()]);
    }

    /// The device it records for.
    pub(crate) fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// This encoder's number: later encoders have larger ones.
    pub(crate) fn id(&self) -> u64 {
        self.id
    }
}

fn encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("frame"),
    })
}

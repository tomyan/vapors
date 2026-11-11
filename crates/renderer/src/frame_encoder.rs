//! One command encoder per frame: every pipeline records its passes
//! into it through a [`Target`](crate::Target), and the frame submits
//! it once. On ANV one `Queue::submit` took about 35 µs
//! (docs/perf/baseline-2026-11-10.md, run 3), about half of each draw
//! call's time when every draw submitted.

use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::ArenaFull;

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
/// pipeline. A pipeline given a newer encoder while an older one it drew
/// for is still open, or an older encoder after a newer one, panics.
///
/// A draw whose bytes would need a buffer over the device's
/// `max_buffer_size` is not recorded; [`FrameEncoder::submit`] returns
/// the first such refusal, and [`FrameEncoder::take_refusal`] hands it
/// over sooner.
pub struct FrameEncoder {
    device: wgpu::Device,
    encoder: RefCell<wgpu::CommandEncoder>,
    id: u64,
    open: Open,
    refusal: RefCell<Option<ArenaFull>>,
}

impl FrameEncoder {
    /// An empty encoder on `device`.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            device: device.clone(),
            encoder: RefCell::new(encoder(device)),
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            open: Open(Arc::new(AtomicBool::new(true))),
            refusal: RefCell::new(None),
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
        submit_timed(queue, done);
    }

    /// Submits everything recorded on `queue`.
    ///
    /// # Errors
    ///
    /// [`ArenaFull`] when a draw was refused and not taken with
    /// [`FrameEncoder::take_refusal`]; the draws recorded are submitted.
    pub fn submit(self, queue: &wgpu::Queue) -> Result<(), ArenaFull> {
        submit_timed(queue, self.encoder.into_inner());
        self.refusal.into_inner().map_or(Ok(()), Err)
    }

    /// The first draw refused since the last take, if any.
    pub fn take_refusal(&self) -> Option<ArenaFull> {
        self.refusal.borrow_mut().take()
    }

    /// Records that a draw was refused; the first refusal is kept.
    pub(crate) fn refuse(&self, refusal: ArenaFull) {
        self.refusal.borrow_mut().get_or_insert(refusal);
    }

    /// Whether this encoder is still open, read after it is gone: false
    /// once it is submitted or dropped.
    pub(crate) fn open_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.open.0)
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

/// Cleared when the encoder is submitted or dropped.
struct Open(Arc<AtomicBool>);

impl Drop for Open {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

/// Finishes `encoder` inside a `draw step` span with phase `encode` and
/// submits it inside one with phase `submit`, for vapors-perf.
fn submit_timed(queue: &wgpu::Queue, encoder: wgpu::CommandEncoder) {
    let commands = {
        let _encoding = tracing::trace_span!("draw step", phase = "encode").entered();
        encoder.finish()
    };
    let _submit = tracing::trace_span!("draw step", phase = "submit").entered();
    queue.submit([commands]);
}

fn encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("frame"),
    })
}

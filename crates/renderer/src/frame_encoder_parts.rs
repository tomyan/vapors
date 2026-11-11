//! A [`FrameEncoder`]'s timing on the GPU, and what it tells the
//! pipelines and the frame about itself.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::FrameEncoder;
use crate::{ArenaFull, EncoderBudget, FrameClock};

impl FrameEncoder {
    /// An empty encoder on `device`, with the default budget, that writes
    /// a timestamp pair around each render pass it begins into `clock`,
    /// resolved into its submit for [`GpuTimer`](crate::GpuTimer) to read.
    #[must_use]
    pub fn timed(device: &wgpu::Device, clock: FrameClock) -> Self {
        let mut encoder = Self::with_budget(device, EncoderBudget::default());
        encoder.clock = Some(clock);
        encoder
    }

    /// Runs `record` with `encoder`, a command encoder of the caller's
    /// that is submitted before this one, timed by a pair of its own
    /// written by empty passes on `view` before and after: for a
    /// `gpu-surface` renderer's passes. Untimed, it runs `record` alone.
    pub fn time_section<R>(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        record: impl FnOnce(&mut wgpu::CommandEncoder) -> R,
    ) -> R {
        match &self.clock {
            Some(clock) => clock.time_section(encoder, view, record),
            None => record(encoder),
        }
    }

    /// The render passes begun since this encoder was made, flushes
    /// included.
    #[must_use]
    pub fn render_passes(&self) -> u64 {
        self.passes.get()
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

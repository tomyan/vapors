//! One command encoder per frame: every pipeline records its passes
//! into it through a [`Target`](crate::Target), and the frame submits
//! it once. On ANV one `Queue::submit` took about 35 µs
//! (docs/perf/baseline-2026-11-10.md, run 3), about half of each draw
//! call's time when every draw submitted.

use std::cell::{Cell, RefCell};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::encoder_budget::Spent;
use crate::open_pass::OpenPass;
use crate::{ArenaFull, EncoderBudget};

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
///
/// Consecutive draws on one view share one render pass. The pass ends
/// when a draw goes to another view, on a clear, on [`FrameEncoder::record`],
/// [`FrameEncoder::flush`] and [`FrameEncoder::submit`]. Before a draw
/// or clear that would pass its [`EncoderBudget`], the encoder flushes.
pub struct FrameEncoder {
    device: wgpu::Device,
    // declared before `encoder`, so a dropped encoder ends its pass first
    pass: RefCell<Option<OpenPass>>,
    encoder: RefCell<wgpu::CommandEncoder>,
    passes: Cell<u64>,
    spent: Spent,
    id: u64,
    open: Open,
    refusal: RefCell<Option<ArenaFull>>,
}

impl FrameEncoder {
    /// An empty encoder on `device`, with the default budget.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        Self::with_budget(device, EncoderBudget::default())
    }

    /// An empty encoder on `device` that flushes at `budget`.
    #[must_use]
    pub fn with_budget(device: &wgpu::Device, budget: EncoderBudget) -> Self {
        Self {
            device: device.clone(),
            pass: RefCell::new(None),
            encoder: RefCell::new(encoder(device)),
            passes: Cell::new(0),
            spent: Spent::new(budget),
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            open: Open(Arc::new(AtomicBool::new(true))),
            refusal: RefCell::new(None),
        }
    }

    /// Runs `record` with the encoder, after ending the open render pass.
    ///
    /// # Panics
    ///
    /// If `record` itself calls `record` on this encoder.
    pub fn record<R>(&self, record: impl FnOnce(&mut wgpu::CommandEncoder) -> R) -> R {
        self.end_pass();
        record(&mut self.encoder.borrow_mut())
    }

    /// The render passes begun since this encoder was made, flushes
    /// included.
    #[must_use]
    pub fn render_passes(&self) -> u64 {
        self.passes.get()
    }

    /// Runs `draw` in the render pass open on `view`, under `scissor`
    /// (`None`: the whole view), beginning one that loads the view if the
    /// open pass is on another view or none is open.
    ///
    /// # Panics
    ///
    /// If `draw` uses this encoder.
    pub(crate) fn draw(
        &self,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        scissor: Option<vapors_scene::Scissor>,
        draw: impl FnOnce(&mut wgpu::RenderPass<'static>),
    ) {
        self.keep_to_budget(queue);
        self.spent.draw();
        let mut open = self.pass.borrow_mut();
        if !open.as_ref().is_some_and(|pass| pass.is_on(view)) {
            *open = None;
            *open = Some(self.begin(view, wgpu::LoadOp::Load));
        }
        draw(open.as_mut().expect("opened above").scissored(scissor));
    }

    /// Clears `view` to `color` in a render pass left open for the draws
    /// on `view` that follow.
    pub(crate) fn clear(&self, queue: &wgpu::Queue, view: &wgpu::TextureView, color: wgpu::Color) {
        self.keep_to_budget(queue);
        self.end_pass();
        let cleared = self.begin(view, wgpu::LoadOp::Clear(color));
        *self.pass.borrow_mut() = Some(cleared);
    }

    /// Submits what is recorded so far and goes on recording into a new
    /// encoder: for a change to a resource that recorded draws read and
    /// later draws must see changed, such as a repacked glyph atlas page.
    pub fn flush(&self, queue: &wgpu::Queue) {
        self.end_pass();
        let done = self.encoder.replace(encoder(&self.device));
        self.spent.reset();
        submit_timed(queue, done);
    }

    /// Submits everything recorded on `queue`.
    ///
    /// # Errors
    ///
    /// [`ArenaFull`] when a draw was refused and not taken with
    /// [`FrameEncoder::take_refusal`]; the draws recorded are submitted.
    pub fn submit(self, queue: &wgpu::Queue) -> Result<(), ArenaFull> {
        self.end_pass();
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

    fn begin(&self, view: &wgpu::TextureView, load: wgpu::LoadOp<wgpu::Color>) -> OpenPass {
        self.passes.set(self.passes.get() + 1);
        self.spent.pass();
        OpenPass::begin(&mut self.encoder.borrow_mut(), view, load)
    }

    fn keep_to_budget(&self, queue: &wgpu::Queue) {
        if self.spent.full() {
            self.flush(queue);
        }
    }

    fn end_pass(&self) {
        self.pass.borrow_mut().take();
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

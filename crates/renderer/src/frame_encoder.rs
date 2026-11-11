//! One command encoder per frame: every pipeline records its passes
//! into it through a [`Target`](crate::Target), and the frame submits
//! it once. On ANV one `Queue::submit` took about 35 µs
//! (docs/perf/baseline-2026-11-10.md, run 3), about half of each draw
//! call's time when every draw submitted.

use std::cell::{Cell, RefCell, RefMut};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::command_encoder::{Open, encoder, submit_timed};
use crate::encoder_budget::Spent;
use crate::frame_clock::FrameClock;
use crate::open_pass::OpenPass;
use crate::{ArenaFull, EncoderBudget, Target};

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
/// that would pass its [`EncoderBudget`]'s draws, or a new pass that
/// would pass its passes, the encoder flushes.
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
    clock: Option<FrameClock>,
}

impl FrameEncoder {
    /// An empty encoder on `device`, with the default budget.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        Self::with_budget(device, EncoderBudget::default())
    }

    /// An empty encoder on `device` that flushes at `budget`.
    ///
    /// # Panics
    ///
    /// If either half of `budget` is 0: no draw could be recorded.
    #[must_use]
    pub fn with_budget(device: &wgpu::Device, budget: EncoderBudget) -> Self {
        assert!(
            budget.passes > 0 && budget.draws > 0,
            "{budget:?}: a budget of 0 passes or draws leaves no room for a draw; \
             give each at least 1, or use FrameEncoder::new for the measured default"
        );
        Self {
            device: device.clone(),
            pass: RefCell::new(None),
            encoder: RefCell::new(encoder(device)),
            passes: Cell::new(0),
            spent: Spent::new(budget),
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            open: Open(Arc::new(AtomicBool::new(true))),
            refusal: RefCell::new(None),
            clock: None,
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

    /// Runs `draw` in the render pass open on `target`'s view, under its
    /// scissor (`None`: the whole viewport), beginning one that loads the
    /// view if the open pass is on another view or none is open.
    ///
    /// # Panics
    ///
    /// If `draw` uses this encoder.
    pub(crate) fn draw(
        &self,
        target: &Target<'_>,
        draw: impl FnOnce(&mut wgpu::RenderPass<'static>),
    ) {
        if self.spent.draws_full() {
            self.flush(target.queue);
        }
        self.spent.draw();
        let mut open = self.pass_on(target.queue, target.view);
        draw(open.scissored(target.scissor, target.viewport));
    }

    /// The pass open on `view`, begun if the open pass is on another
    /// view or none is open.
    fn pass_on(&self, queue: &wgpu::Queue, view: &wgpu::TextureView) -> RefMut<'_, OpenPass> {
        let is_open = self
            .pass
            .borrow()
            .as_ref()
            .is_some_and(|pass| pass.is_on(view));
        if !is_open {
            self.begin(queue, view, wgpu::LoadOp::Load);
        }
        RefMut::map(self.pass.borrow_mut(), |open| {
            open.as_mut().expect("begun above")
        })
    }

    /// Clears `view` to `color` in a render pass left open for the draws
    /// on `view` that follow.
    pub(crate) fn clear(&self, queue: &wgpu::Queue, view: &wgpu::TextureView, color: wgpu::Color) {
        self.begin(queue, view, wgpu::LoadOp::Clear(color));
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
        let mut last = self.encoder.into_inner();
        if let Some(clock) = &self.clock {
            clock.resolve(&mut last);
        }
        submit_timed(queue, last);
        if let Some(clock) = self.clock {
            clock.submitted();
        }
        self.refusal.into_inner().map_or(Ok(()), Err)
    }

    /// Ends the open pass and begins one on `view`, flushing first when
    /// another pass would pass the budget.
    fn begin(
        &self,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        load: wgpu::LoadOp<wgpu::Color>,
    ) {
        self.end_pass();
        if self.spent.passes_full() {
            self.flush(queue);
        }
        self.passes.set(self.passes.get() + 1);
        self.spent.pass();
        let writes = self.clock.as_ref().and_then(FrameClock::pass_writes);
        let pass = OpenPass::begin(&mut self.encoder.borrow_mut(), view, load, writes);
        *self.pass.borrow_mut() = Some(pass);
    }

    fn end_pass(&self) {
        self.pass.borrow_mut().take();
    }
}

#[path = "frame_encoder_parts.rs"]
mod parts;

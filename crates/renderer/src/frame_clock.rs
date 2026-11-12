//! One frame's timestamp pairs: a pair around each render pass its
//! [`FrameEncoder`](crate::FrameEncoder) begins and each section it
//! times, resolved into the frame's last submit and mapped for a later
//! frame to read ([`GpuTimer`](crate::GpuTimer)).

use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc;

use crate::gpu_time::{FrameReading, GpuMissing};
use crate::gpu_timer::Slot;
use crate::gpu_timer_slot::{SlotState, missing};

/// Bytes of one timestamp pair.
pub(crate) const PAIR_BYTES: u64 = 16;

/// The timestamp pairs of one frame, from [`GpuTimer::frame`](crate::GpuTimer::frame),
/// given to [`FrameEncoder::timed`](crate::FrameEncoder::timed). Dropped
/// unsubmitted, the frame's reading is [`GpuMissing::NotSubmitted`].
pub struct FrameClock {
    slot: Rc<Slot>,
    frame: u64,
    capacity: u32,
    /// Pairs asked for, over the capacity included.
    pairs: Cell<u32>,
    /// Sections timed whose encoders have not been submitted.
    unsubmitted: Cell<u32>,
    submitted: bool,
}

impl FrameClock {
    pub(crate) fn new(slot: Rc<Slot>, frame: u64, capacity: u32) -> Self {
        *slot.state.borrow_mut() = SlotState::Recording { frame };
        Self {
            slot,
            frame,
            capacity,
            pairs: Cell::new(0),
            unsubmitted: Cell::new(0),
            submitted: false,
        }
    }

    /// The writes for a render pass about to begin; `None` past the
    /// capacity, which makes the frame's reading missing.
    pub(crate) fn pass_writes(&self) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let pair = self.next_pair()?;
        Some(writes(
            &self.slot.queries,
            Some(pair * 2),
            Some(pair * 2 + 1),
        ))
    }

    /// Runs `record`, between an empty pass on `view` that writes a
    /// pair's start and one that writes its end. Until
    /// [`FrameClock::section_submitted`], the section is unsubmitted.
    pub(crate) fn time_section<R>(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        record: impl FnOnce(&mut wgpu::CommandEncoder) -> R,
    ) -> R {
        self.unsubmitted.set(self.unsubmitted.get() + 1);
        let Some(pair) = self.next_pair() else {
            return record(encoder);
        };
        marker_pass(
            encoder,
            view,
            writes(&self.slot.queries, Some(pair * 2), None),
        );
        let recorded = record(encoder);
        marker_pass(
            encoder,
            view,
            writes(&self.slot.queries, None, Some(pair * 2 + 1)),
        );
        recorded
    }

    /// Records that a timed section's encoder was submitted.
    pub(crate) fn section_submitted(&self) {
        self.unsubmitted
            .set(self.unsubmitted.get().saturating_sub(1));
    }

    /// Records the copy of the frame's timestamps into the read-back
    /// buffer, into the frame's last encoder before it is finished.
    pub(crate) fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        // only a frame whose every pair was written is read back, so
        // no stale tick from an earlier frame is copied
        if self.unread().is_some() {
            return;
        }
        let pairs = self.pairs.get();
        let slot = &self.slot;
        encoder.resolve_query_set(&slot.queries, 0..pairs * 2, &slot.resolved, 0);
        let bytes = u64::from(pairs) * PAIR_BYTES;
        encoder.copy_buffer_to_buffer(&slot.resolved, 0, &slot.readable, 0, bytes);
    }

    /// After the frame's last submit: maps what [`FrameClock::resolve`]
    /// copied, or records the reading known without it.
    pub(crate) fn submitted(mut self) {
        self.submitted = true;
        let state = match self.unread() {
            Some(reading) => SlotState::Done(reading),
            None => self.map(),
        };
        *self.slot.state.borrow_mut() = state;
    }

    /// The frame's reading when there is nothing to read back: no pairs,
    /// too many, or a section's ticks never written.
    fn unread(&self) -> Option<FrameReading> {
        let (frame, pairs, capacity) = (self.frame, self.pairs.get(), self.capacity);
        let why = match (pairs, self.unsubmitted.get()) {
            (0, 0) => return Some(zero(frame)),
            (_, 0) if pairs <= capacity => return None,
            (_, 0) => GpuMissing::OverCapacity { pairs, capacity },
            (_, sections) => GpuMissing::SectionNotSubmitted { sections },
        };
        Some(missing(frame, pairs, why))
    }

    fn map(&self) -> SlotState {
        let (sender, outcome) = mpsc::channel();
        let bytes = u64::from(self.pairs.get()) * PAIR_BYTES;
        self.slot
            .readable
            .map_async(wgpu::MapMode::Read, 0..bytes, move |result| {
                // a reclaimed slot has dropped the receiver; nothing waits
                let _ = sender.send(result);
            });
        SlotState::Mapping {
            frame: self.frame,
            pairs: self.pairs.get(),
            outcome,
        }
    }

    fn next_pair(&self) -> Option<u32> {
        let pair = self.pairs.get();
        self.pairs.set(pair + 1);
        (pair < self.capacity).then_some(pair)
    }
}

impl Drop for FrameClock {
    fn drop(&mut self) {
        if !self.submitted {
            let reading = missing(self.frame, self.pairs.get(), GpuMissing::NotSubmitted);
            *self.slot.state.borrow_mut() = SlotState::Done(reading);
        }
    }
}

fn zero(frame: u64) -> FrameReading {
    FrameReading {
        frame,
        pairs: 0,
        time: Ok(std::time::Duration::ZERO),
    }
}

fn writes(
    query_set: &wgpu::QuerySet,
    start: Option<u32>,
    end: Option<u32>,
) -> wgpu::RenderPassTimestampWrites<'_> {
    wgpu::RenderPassTimestampWrites {
        query_set,
        beginning_of_pass_write_index: start,
        end_of_pass_write_index: end,
    }
}

/// A pass on `view` that draws nothing and keeps its contents, for its
/// timestamp alone.
fn marker_pass(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    writes: wgpu::RenderPassTimestampWrites<'_>,
) {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("timestamp"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: Some(writes),
        occlusion_query_set: None,
        multiview_mask: None,
    });
}

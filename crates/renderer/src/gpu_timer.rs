//! The GPU's time per frame, from timestamp queries, read back without
//! waiting: each frame writes into one slot of a ring, and the frames
//! after it read the slot once the GPU has mapped it.

use std::cell::RefCell;
use std::rc::Rc;

use crate::frame_clock::{FrameClock, PAIR_BYTES};
use crate::gpu_time::{FrameReading, GpuMissing, GpuReading, GpuResolution, GpuTime, sum_pairs};
use crate::gpu_timer_slot::{Finished, SlotState};

/// Frames whose timestamps may be on their way back at once. A
/// reading not mapped when its slot comes round again, this many frames
/// later, is missing ([`GpuMissing::Deadline`] names the frames). The
/// number is reasoned from a configuration default, not measured: a
/// native window's surface keeps at most 2 frames queued
/// (`desired_maximum_frame_latency`, 2 in wgpu's default configuration,
/// which vapors-platform uses), so a frame's GPU work has finished by
/// the time the third after it begins; 4 leaves one frame spare. On the
/// web the browser sets the latency, not this configuration.
const SLOTS: usize = 4;

/// Timestamp pairs one frame may write by default. The app's painter
/// writes one per render pass (1 per frame in every vapors-perf
/// workload, docs/perf/baseline-2026-11-10.md) and one per `gpu-surface`
/// rendered.
pub const DEFAULT_PAIRS: u32 = 64;

/// One frame's query set and the buffers its timestamps come back in.
pub(crate) struct Slot {
    pub(crate) queries: wgpu::QuerySet,
    pub(crate) resolved: wgpu::Buffer,
    pub(crate) readable: wgpu::Buffer,
    pub(crate) state: RefCell<SlotState>,
}

/// Times frames on the GPU where the device has `TIMESTAMP_QUERY`.
pub struct GpuTimer {
    device: wgpu::Device,
    /// Nanoseconds per tick.
    period: f64,
    capacity: u32,
    slots: Vec<Rc<Slot>>,
    begun: usize,
    latest: Option<FrameReading>,
    resolution: GpuResolution,
}

/// What a device needs for a [`GpuTimer`]: timestamps written at the
/// start and end of passes.
pub const TIMER_FEATURES: wgpu::Features = wgpu::Features::TIMESTAMP_QUERY;

impl GpuTimer {
    /// A timer of [`DEFAULT_PAIRS`] pairs per frame; `None` when
    /// `device` lacks `TIMESTAMP_QUERY`.
    #[must_use]
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        Self::with_capacity(device, queue, DEFAULT_PAIRS)
    }

    /// A timer of `pairs` timestamp pairs per frame; `None` when
    /// `device` lacks `TIMESTAMP_QUERY`.
    #[must_use]
    pub fn with_capacity(device: &wgpu::Device, queue: &wgpu::Queue, pairs: u32) -> Option<Self> {
        if !device.features().contains(TIMER_FEATURES) {
            return None;
        }
        Some(Self {
            device: device.clone(),
            period: f64::from(queue.get_timestamp_period()),
            capacity: pairs,
            slots: (0..SLOTS).map(|_| Rc::new(slot(device, pairs))).collect(),
            begun: 0,
            latest: None,
            resolution: GpuResolution::of_period(queue.get_timestamp_period()),
        })
    }

    /// The latest frame read back, or [`GpuTime::Pending`] before one is.
    #[must_use]
    pub fn time(&self) -> GpuTime {
        let reading = |latest: &FrameReading| GpuReading {
            frame: latest.frame,
            pairs: latest.pairs,
            time: latest.time.clone(),
            resolution: self.resolution,
        };
        self.latest
            .as_ref()
            .map_or(GpuTime::Pending, |latest| GpuTime::Read(reading(latest)))
    }

    /// The clock for the frame `index`, after reading back the frames
    /// the GPU has finished, without waiting for it.
    pub fn frame(&mut self, index: u64) -> FrameClock {
        self.poll();
        let slot = Rc::clone(&self.slots[self.begun % SLOTS]);
        let reclaimed = slot.state.borrow_mut().reclaim(SLOTS);
        if let Some(reading) = reclaimed {
            slot.readable.unmap();
            self.keep(reading);
        }
        self.begun += 1;
        FrameClock::new(slot, index, self.capacity)
    }

    /// Reads back every slot the GPU has mapped, oldest first, keeping
    /// the newest reading.
    pub fn poll(&mut self) {
        // Poll does not wait, so it has no timeout to report; a lost
        // device fails the mapping, which the reading reports
        let _ = self.device.poll(wgpu::PollType::Poll);
        for offset in 0..SLOTS {
            let slot = Rc::clone(&self.slots[(self.begun + offset) % SLOTS]);
            if let Some(reading) = self.finished(&slot) {
                self.keep(reading);
            }
        }
    }

    fn finished(&self, slot: &Slot) -> Option<FrameReading> {
        let finished = slot.state.borrow_mut().take_finished();
        match finished {
            Finished::Nothing => None,
            Finished::Reading(reading) => Some(reading),
            Finished::Mapped { frame, pairs } => Some(self.read(slot, frame, pairs)),
        }
    }

    fn read(&self, slot: &Slot, frame: u64, pairs: u32) -> FrameReading {
        let ticks = ticks(&slot.readable, pairs);
        slot.readable.unmap();
        FrameReading {
            frame,
            pairs,
            time: ticks.and_then(|ticks| sum_pairs(&ticks, self.period)),
        }
    }

    fn keep(&mut self, reading: FrameReading) {
        if self
            .latest
            .as_ref()
            .is_none_or(|kept| kept.frame <= reading.frame)
        {
            self.latest = Some(reading);
        }
    }
}

/// The first `pairs` pairs' ticks of the mapped `readable`.
fn ticks(readable: &wgpu::Buffer, pairs: u32) -> Result<Vec<u64>, GpuMissing> {
    let bytes = readable
        .get_mapped_range(..u64::from(pairs) * PAIR_BYTES)
        .map_err(|error| GpuMissing::MapFailed(error.to_string()))?;
    let chunks = bytes.as_chunks::<8>().0;
    Ok(chunks.iter().map(|b| u64::from_le_bytes(*b)).collect())
}

fn slot(device: &wgpu::Device, pairs: u32) -> Slot {
    let bytes = u64::from(pairs) * PAIR_BYTES;
    Slot {
        queries: device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("frame timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: pairs * 2,
        }),
        resolved: buffer(
            device,
            bytes,
            wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        ),
        readable: buffer(
            device,
            bytes,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        ),
        state: RefCell::new(SlotState::Free),
    }
}

fn buffer(device: &wgpu::Device, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("frame timestamps"),
        size,
        usage,
        mapped_at_creation: false,
    })
}

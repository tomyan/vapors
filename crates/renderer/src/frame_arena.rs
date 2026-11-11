//! A pipeline's buffer for one frame's draws: each draw's bytes at their
//! own offset, so draws recorded into one [`FrameEncoder`] and submitted
//! together each read their own data. `Queue::write_buffer` takes effect
//! before the next submit, so two draws written at one offset would both
//! read the second's bytes.
//!
//! Buffers grow and never shrink: an arena's buffer is as large as the
//! most bytes one frame wrote through its pipeline, up to the device's
//! `max_buffer_size`, past which the frame goes on in another buffer of
//! that size. A single draw larger than `max_buffer_size` is refused
//! ([`ArenaFull`]).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{ArenaFull, FrameEncoder, WorkCounter};

/// Where a draw's bytes went: a buffer (kept alive by the passes that
/// use it, though the arena moves to a larger one) and the range.
pub struct Placed {
    /// The buffer holding the bytes.
    pub buffer: wgpu::Buffer,
    /// Their offset in it.
    pub offset: u64,
    /// Their length.
    pub size: u64,
}

impl Placed {
    /// The bytes, for `set_vertex_buffer`.
    #[must_use]
    pub fn slice(&self) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(self.offset..self.offset + self.size)
    }

    /// The bytes, for a bind group entry.
    #[must_use]
    pub fn binding(&self) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: &self.buffer,
            offset: self.offset,
            size: wgpu::BufferSize::new(self.size),
        })
    }
}

/// One growing buffer, filled from offset 0 again by each new encoder.
pub struct FrameArena {
    label: &'static str,
    usage: wgpu::BufferUsages,
    align: u64,
    buffer: wgpu::Buffer,
    used: u64,
    encoder: u64,
    open: Arc<AtomicBool>,
}

impl FrameArena {
    /// An arena of `usage` (`COPY_DST` is added) whose draws start at
    /// multiples of `align`.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        label: &'static str,
        usage: wgpu::BufferUsages,
        align: u64,
    ) -> Self {
        let usage = usage | wgpu::BufferUsages::COPY_DST;
        let first = (64 * 1024).min(device.limits().max_buffer_size);
        Self {
            label,
            usage,
            align: align.max(wgpu::COPY_BUFFER_ALIGNMENT),
            buffer: buffer(device, label, usage, first),
            used: 0,
            encoder: 0,
            open: Arc::new(AtomicBool::new(false)),
        }
    }

    /// An arena for uniforms: draws start at the device's uniform
    /// offset alignment.
    #[must_use]
    pub fn uniforms(device: &wgpu::Device, label: &'static str) -> Self {
        let align = device.limits().min_uniform_buffer_offset_alignment;
        Self::new(device, label, wgpu::BufferUsages::UNIFORM, u64::from(align))
    }

    /// Writes `bytes` (a multiple of 4 long) for a draw recorded into
    /// `frame`, counting them in `work`. `None` when they would need a
    /// buffer over the device's `max_buffer_size`: the refusal is kept
    /// on `frame` and the draw must not be recorded.
    ///
    /// # Panics
    ///
    /// If `frame` is older than an encoder this arena already wrote for,
    /// or newer while that encoder is still open (neither submitted nor
    /// dropped): either way one encoder's bytes would overwrite the
    /// other's.
    pub fn push(
        &mut self,
        frame: &FrameEncoder,
        queue: &wgpu::Queue,
        work: &WorkCounter,
        bytes: &[u8],
    ) -> Option<Placed> {
        self.begin(frame);
        let size = bytes.len() as u64;
        let offset = self.offset_for(frame, size)?;
        work.write_buffer(queue, &self.buffer, offset, bytes);
        self.used = offset + size;
        let buffer = self.buffer.clone();
        Some(Placed {
            buffer,
            offset,
            size,
        })
    }

    /// Where `size` bytes go: after the last draw's, or at 0 in a larger
    /// buffer; `None`, with the refusal kept on `frame`, when no buffer
    /// the device allows holds them.
    fn offset_for(&mut self, frame: &FrameEncoder, size: u64) -> Option<u64> {
        let offset = self.used.next_multiple_of(self.align);
        if offset + size <= self.buffer.size() {
            return Some(offset);
        }
        match self.grow(frame.device(), size) {
            Ok(()) => Some(0),
            Err(refusal) => {
                frame.refuse(refusal);
                None
            }
        }
    }

    fn begin(&mut self, frame: &FrameEncoder) {
        assert!(
            frame.id() >= self.encoder,
            "{}: a draw for encoder {} after encoder {}, an older encoder after a newer one; \
             submit each FrameEncoder before drawing with the next",
            self.label,
            frame.id(),
            self.encoder
        );
        if frame.id() > self.encoder {
            self.start(frame);
        }
    }

    /// Starts again at offset 0 for `frame`, once the encoder drawn for
    /// before is closed.
    fn start(&mut self, frame: &FrameEncoder) {
        assert!(
            !self.open.load(Ordering::Relaxed),
            "{}: a draw for encoder {} while encoder {}, which this pipeline drew for, is still open; \
             submit or drop each FrameEncoder before drawing with the next",
            self.label,
            frame.id(),
            self.encoder
        );
        (self.encoder, self.used, self.open) = (frame.id(), 0, frame.open_flag());
    }

    /// Moves to a buffer twice the size, or `at_least`, up to the
    /// device's `max_buffer_size`; draws already recorded keep the old
    /// one. Refused when `at_least` is over it.
    fn grow(&mut self, device: &wgpu::Device, at_least: u64) -> Result<(), ArenaFull> {
        let limit = device.limits().max_buffer_size;
        let needed = at_least.next_multiple_of(self.align);
        if needed > limit {
            return Err(ArenaFull {
                pipeline: self.label,
                size: needed,
                limit,
            });
        }
        let size = (self.buffer.size() * 2).clamp(needed, limit);
        self.buffer = buffer(device, self.label, self.usage, size);
        Ok(())
    }
}

fn buffer(
    device: &wgpu::Device,
    label: &str,
    usage: wgpu::BufferUsages,
    size: u64,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage,
        mapped_at_creation: false,
    })
}

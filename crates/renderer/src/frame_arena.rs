//! A pipeline's buffer for one frame's draws: each draw's bytes at their
//! own offset, so draws recorded into one [`FrameEncoder`] and submitted
//! together each read their own data. `Queue::write_buffer` takes effect
//! before the next submit, so two draws written at one offset would both
//! read the second's bytes.

use crate::{FrameEncoder, WorkCounter};

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
        Self {
            label,
            usage,
            align: align.max(wgpu::COPY_BUFFER_ALIGNMENT),
            buffer: buffer(device, label, usage, 64 * 1024),
            used: 0,
            encoder: 0,
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
    /// `frame`, counting them in `work`.
    ///
    /// # Panics
    ///
    /// If `frame` is older than an encoder this arena already wrote for:
    /// its draws' bytes may have been overwritten.
    pub fn push(
        &mut self,
        frame: &FrameEncoder,
        queue: &wgpu::Queue,
        work: &WorkCounter,
        bytes: &[u8],
    ) -> Placed {
        self.begin(frame);
        let size = bytes.len() as u64;
        let mut offset = self.used.next_multiple_of(self.align);
        if offset + size > self.buffer.size() {
            self.grow(frame.device(), size);
            offset = 0;
        }
        work.write_buffer(queue, &self.buffer, offset, bytes);
        self.used = offset + size;
        let buffer = self.buffer.clone();
        Placed {
            buffer,
            offset,
            size,
        }
    }

    fn begin(&mut self, frame: &FrameEncoder) {
        assert!(
            frame.id() >= self.encoder,
            "{}: a draw for an encoder older than one already drawn with; \
             submit each FrameEncoder before drawing with the next",
            self.label
        );
        if frame.id() > self.encoder {
            (self.encoder, self.used) = (frame.id(), 0);
        }
    }

    /// Moves to a buffer twice the size, or `at_least`; draws already
    /// recorded keep the old one.
    fn grow(&mut self, device: &wgpu::Device, at_least: u64) {
        let size = (self.buffer.size() * 2).max(at_least.next_multiple_of(self.align));
        self.buffer = buffer(device, self.label, self.usage, size);
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

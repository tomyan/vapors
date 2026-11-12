//! Moving a [`Retained`] buffer's live ranges into a new buffer, packed
//! from offset 0: compacted, or grown (`retained.rs`).

use super::{ALIGN, Retained};
use crate::free_ranges::FreeRanges;
use crate::{ArenaFull, FrameEncoder, Holding, WorkCounter};

impl Retained {
    /// The start of `size` bytes after moving the live ranges into a
    /// buffer with room for them ([`Retained::move_size`]). `None`, with
    /// the refusal kept on `frame`, when that is over the device's
    /// `max_buffer_size`.
    pub(super) fn moved(
        &mut self,
        frame: &FrameEncoder,
        work: &WorkCounter,
        size: u64,
    ) -> Option<u64> {
        let limit = frame.device().limits().max_buffer_size;
        let (to, compacts) = self.move_size(size, limit);
        if to > limit {
            let size = (self.live + size).next_multiple_of(ALIGN);
            let (pipeline, holding) = (self.label, Holding::Kept);
            frame.refuse(ArenaFull {
                pipeline,
                size,
                limit,
                holding,
            });
            return None;
        }
        self.move_to(frame, to);
        work.retained_moved(compacts);
        self.free.take(size)
    }

    /// The size of the buffer to move the live ranges and `size` more
    /// bytes into, and whether it is the same size: it is when they fit
    /// and at least half of it is free after the move (ranges released
    /// for this encoder are left behind), else twice the size up to
    /// `limit`, or what they need.
    fn move_size(&self, size: u64, limit: u64) -> (u64, bool) {
        let now = self.buffer.size();
        let needed = self.live + size;
        if needed <= now && now - self.live >= now / 2 {
            return (now, true);
        }
        ((now * 2).min(limit).max(needed), false)
    }

    /// Copies each live range into a new buffer of `size` bytes, packed
    /// in slot order, and reads them from there. No draw recorded so far
    /// reads the new buffer, so all of it past the live ranges is free.
    fn move_to(&mut self, frame: &FrameEncoder, size: u64) {
        let moved = buffer(frame.device(), self.label, self.usage, size);
        let mut end = 0;
        frame.record(|encoder| {
            for range in self.ranges.iter_mut().flatten() {
                let len = range.end - range.start;
                encoder.copy_buffer_to_buffer(&self.buffer, range.start, &moved, end, len);
                *range = end..end + len;
                end += len;
            }
        });
        (self.buffer, self.free) = (moved, FreeRanges::after(end, size));
        // The released ranges were left behind in the old buffer.
        self.released.clear();
    }
}

/// A buffer of `size` bytes for `usage`.
pub(super) fn buffer(
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

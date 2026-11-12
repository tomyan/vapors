//! Moving a [`Retained`] buffer's live ranges into a new buffer, packed
//! from offset 0: compacted, or grown (`retained.rs`).

use super::{ALIGN, Retained, buffer};
use crate::free_ranges::FreeRanges;
use crate::{ArenaFull, FrameEncoder, WorkCounter};

impl Retained {
    /// The start of `size` bytes after moving the live ranges into a
    /// buffer with room for them: of the same size when at least half
    /// of it is free, else twice the size or what they need. `None`,
    /// with the refusal kept on `frame`, when that is over the device's
    /// `max_buffer_size`.
    pub(super) fn moved(
        &mut self,
        frame: &FrameEncoder,
        work: &WorkCounter,
        size: u64,
    ) -> Option<u64> {
        let now = self.buffer.size();
        let needed = self.live + size;
        let compacts = self.free.free_bytes() >= now / 2 && needed <= now;
        let limit = frame.device().limits().max_buffer_size;
        let to = if compacts {
            now
        } else {
            (now * 2).min(limit).max(needed)
        };
        if to > limit {
            frame.refuse(ArenaFull {
                pipeline: self.label,
                size: needed.next_multiple_of(ALIGN),
                limit,
            });
            return None;
        }
        self.move_to(frame, to);
        work.retained_moved(compacts);
        self.free.take(size)
    }

    /// Copies each live range into a new buffer of `size` bytes, packed
    /// in slot order, and reads them from there.
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
    }
}

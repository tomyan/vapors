//! A pipeline's buffer for bytes kept between frames (IU4,
//! `docs/plans/iu4.md` section 2): each kept range written once and
//! drawn from by later frames until it is released. Free ranges are
//! taken first fit and merged when released ([`FreeRanges`]).
//!
//! When no free range holds a new range, the live ranges move into a
//! new buffer, packed from offset 0 by copies recorded into the frame's
//! encoder: compacted into one of the same size when at least half of
//! it is free after the move, else grown to twice the size (or what the bytes
//! need), up to the device's `max_buffer_size`, past which the bytes
//! are refused ([`ArenaFull`]). Draws recorded before the move keep the
//! old buffer; the copies run after them and before later draws, and
//! every write is to a range no copy touches.
//!
//! A released range may still be read by a draw recorded into the
//! encoder the buffer last wrote or drew for, so it is free for new
//! bytes only once a newer encoder writes or draws; writes or draws for
//! a newer encoder while an older one is open panic as
//! [`FrameArena::push`](crate::FrameArena::push) does.

use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::encoder_turn::EncoderTurn;
use crate::free_ranges::FreeRanges;
use crate::kept_range::KeptRange;
use crate::{FrameEncoder, WorkCounter};

/// Kept ranges start at multiples of this, and are multiples of it long.
const ALIGN: u64 = 16;
/// A new buffer's size, as [`FrameArena`](crate::FrameArena)'s first.
const FIRST_SIZE: u64 = 64 * 1024;

/// Each buffer's number, so a range is only read from its own.
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);

/// One growing buffer of kept ranges.
pub struct Retained {
    label: &'static str,
    usage: wgpu::BufferUsages,
    buffer: wgpu::Buffer,
    free: FreeRanges,
    /// By [`KeptRange::slot`]: the range's bytes, or `None` once released.
    ranges: Vec<Option<Range<u64>>>,
    vacant: Vec<u32>,
    /// Ranges released since the last encoder began: free once a newer
    /// one writes or draws.
    released: Vec<Range<u64>>,
    live: u64,
    owner: u64,
    turn: EncoderTurn,
}

impl Retained {
    /// An empty buffer of `usage` (`COPY_DST` and `COPY_SRC` are added).
    #[must_use]
    pub fn new(device: &wgpu::Device, label: &'static str, usage: wgpu::BufferUsages) -> Self {
        let usage = usage | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC;
        let size = FIRST_SIZE.min(device.limits().max_buffer_size);
        Self {
            label,
            usage,
            buffer: buffer(device, label, usage, size),
            free: FreeRanges::new(size),
            ranges: Vec::new(),
            vacant: Vec::new(),
            released: Vec::new(),
            live: 0,
            owner: NEXT_OWNER.fetch_add(1, Ordering::Relaxed),
            turn: EncoderTurn::new(label),
        }
    }

    /// Writes `bytes` into a free range for draws recorded into `frame`
    /// and later ones, counting them in `work`; moving the live ranges
    /// first when no free range holds them. `None` when the buffer they
    /// need is over the device's `max_buffer_size`: the refusal is kept
    /// on `frame` and nothing is drawn from them.
    ///
    /// # Panics
    ///
    /// If `bytes` is empty or not a multiple of 16 long; as
    /// [`FrameArena::push`](crate::FrameArena::push) for encoders.
    pub fn keep(
        &mut self,
        frame: &FrameEncoder,
        queue: &wgpu::Queue,
        work: &WorkCounter,
        bytes: &[u8],
    ) -> Option<KeptRange> {
        let size = self.aligned(bytes);
        self.begin(frame);
        let start = match self.free.take(size) {
            Some(start) => start,
            None => self.moved(frame, work, size)?,
        };
        work.write_buffer(queue, &self.buffer, start, bytes);
        self.live += size;
        Some(self.hold(start..start + size))
    }

    /// The kept bytes of `kept`, for `set_vertex_buffer` in a draw
    /// recorded into `frame`: ranges released from now on stay unused
    /// until a newer encoder writes or draws.
    ///
    /// # Panics
    ///
    /// If `kept` is another buffer's range; as
    /// [`FrameArena::push`](crate::FrameArena::push) for encoders.
    #[must_use]
    pub fn slice(&mut self, frame: &FrameEncoder, kept: &KeptRange) -> wgpu::BufferSlice<'_> {
        self.begin(frame);
        self.buffer.slice(self.range(kept))
    }

    /// The kept bytes of each of `kept`, for draws recorded into
    /// `frame`, as [`Retained::slice`] gives one.
    ///
    /// # Panics
    ///
    /// As [`Retained::slice`].
    pub fn slices<'a>(
        &'a mut self,
        frame: &FrameEncoder,
        kept: &[&KeptRange],
    ) -> Vec<wgpu::BufferSlice<'a>> {
        self.begin(frame);
        let ranges: Vec<Range<u64>> = kept.iter().map(|kept| self.range(kept)).collect();
        ranges
            .into_iter()
            .map(|range| self.buffer.slice(range))
            .collect()
    }

    /// How many bytes `kept` holds.
    ///
    /// # Panics
    ///
    /// If `kept` is another buffer's range.
    #[must_use]
    pub fn len(&self, kept: &KeptRange) -> u64 {
        let range = self.range(kept);
        range.end - range.start
    }

    /// Gives `kept`'s bytes back, for ranges kept once a newer encoder
    /// than the last one written or drawn for begins: a draw recorded
    /// before may still read them.
    ///
    /// # Panics
    ///
    /// If `kept` is another buffer's range.
    #[allow(clippy::needless_pass_by_value)] // taken, so a released range is never used again
    pub fn release(&mut self, kept: KeptRange) {
        let range = self.range(&kept);
        self.ranges[kept.slot as usize] = None;
        self.vacant.push(kept.slot);
        self.live -= range.end - range.start;
        self.released.push(range);
    }

    /// Bytes held by kept ranges.
    #[must_use]
    pub fn live_bytes(&self) -> u64 {
        self.live
    }

    /// How many `bytes` there are.
    ///
    /// # Panics
    ///
    /// If none, or not a multiple of 16.
    fn aligned(&self, bytes: &[u8]) -> u64 {
        let size = bytes.len() as u64;
        assert!(
            size > 0 && size.is_multiple_of(ALIGN),
            "{}: kept bytes are {size} long, not a non-zero multiple of {ALIGN}; pad each \
             instance or vertex to 16 bytes (retained.rs)",
            self.label
        );
        size
    }

    /// Takes writes and draws for `frame`: when it is a newer encoder,
    /// the ranges released for older ones are free.
    fn begin(&mut self, frame: &FrameEncoder) {
        if self.turn.begin(frame) {
            let free = &mut self.free;
            self.released.drain(..).for_each(|range| free.give(range));
        }
    }

    /// `range` held under a slot of its own.
    fn hold(&mut self, range: Range<u64>) -> KeptRange {
        let slot = self.vacant.pop().unwrap_or_else(|| {
            self.ranges.push(None);
            u32::try_from(self.ranges.len() - 1).expect("fewer kept ranges than u32::MAX")
        });
        self.ranges[slot as usize] = Some(range);
        KeptRange {
            owner: self.owner,
            slot,
        }
    }

    fn range(&self, kept: &KeptRange) -> Range<u64> {
        assert_eq!(
            kept.owner, self.owner,
            "{}: a range kept by another Retained buffer read from this one; release and \
             draw each KeptRange through the pipeline that kept it (retained.rs)",
            self.label
        );
        self.ranges[kept.slot as usize]
            .clone()
            .expect("a KeptRange is held until released, which consumes it")
    }
}

#[path = "retained_move.rs"]
mod moved;
use moved::buffer;

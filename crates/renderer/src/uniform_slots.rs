//! A pipeline's uniform values kept on the GPU between frames: each
//! distinct value (16 bytes, or `SIZE`) in its own slot of one buffer,
//! written when it first appears and bound after by its dynamic offset,
//! found by its bytes in a map. A frame whose values the buffer already
//! holds writes nothing; a bind group made for the buffer serves every
//! slot.

use crate::encoder_turn::EncoderTurn;
use crate::{ArenaFull, FrameEncoder, Holding, RowOffsets, WorkCounter};

/// Slots in a new buffer: 4 KiB at the 256-byte alignment WebGPU, ANV
/// and lavapipe give. A frame needing more moves to a buffer twice as
/// large.
const FIRST_SLOTS: u64 = 16;

/// One uniform buffer of slots, each a whole number of the device's
/// uniform offset alignments apart, each holding a value of `SIZE`
/// bytes: the viewport, a scroll frame's shift and its rows' offsets,
/// or the text blend parameters with them.
pub struct UniformSlots<const SIZE: usize = 16> {
    label: &'static str,
    /// The bytes from one slot to the next: `SIZE` rounded up to the
    /// device's uniform offset alignment.
    align: u64,
    buffer: wgpu::Buffer,
    slots: Vec<Slot<SIZE>>,
    /// The slots holding values of each hash (`word_hash`).
    by_hash: std::collections::HashMap<u64, Vec<usize>, WordHashing>,
    turn: EncoderTurn,
    /// Counts the buffers made; a bind group is for one buffer.
    generation: u64,
}

/// Whether a held value is the one looked up.
type Same<'a, const SIZE: usize> = dyn Fn(&[u8; SIZE]) -> bool + 'a;

/// A slot's value, its hash, and the last encoder whose draw read it.
struct Slot<const SIZE: usize> {
    value: [u8; SIZE],
    hash: u64,
    encoder: u64,
}

impl<const SIZE: usize> UniformSlots<SIZE> {
    /// An empty buffer of `FIRST_SLOTS` slots for the pipeline `label`.
    ///
    /// # Panics
    ///
    /// When `SIZE` is not a whole number of 16-byte rows (WGSL's uniform
    /// struct alignment).
    #[must_use]
    pub fn new(device: &wgpu::Device, label: &'static str) -> Self {
        const {
            assert!(
                SIZE.is_multiple_of(16),
                "a uniform value is whole 16-byte rows"
            );
        }
        let alignment = u64::from(device.limits().min_uniform_buffer_offset_alignment);
        let align = (SIZE as u64).div_ceil(alignment) * alignment;
        Self {
            label,
            align,
            buffer: buffer(device, label, first_size(device, align)),
            slots: Vec::new(),
            by_hash: std::collections::HashMap::default(),
            turn: EncoderTurn::new(label),
            generation: 1,
        }
    }

    /// One slot's worth of the buffer, for a bind group entry whose
    /// layout has a dynamic offset.
    #[must_use]
    pub fn binding(&self) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: &self.buffer,
            offset: 0,
            size: wgpu::BufferSize::new(SIZE as u64),
        })
    }

    /// Which buffer [`UniformSlots::binding`] names: a bind group made
    /// for an earlier generation binds a buffer no later slot is in.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The dynamic offset of the slot holding `value` for a draw
    /// recorded into `frame`; written, counted in `work`, into a slot no
    /// draw of `frame` reads when none holds it. `None` when a buffer
    /// with room would be over the device's `max_buffer_size`: the
    /// refusal is kept on `frame` and the draw must not be recorded.
    ///
    /// # Panics
    ///
    /// As [`FrameArena::push`](crate::FrameArena::push): an older
    /// encoder after a newer one, or a newer one while the older is open.
    pub fn slot(
        &mut self,
        frame: &FrameEncoder,
        queue: &wgpu::Queue,
        work: &WorkCounter,
        value: [u8; SIZE],
    ) -> Option<u32> {
        let same = |held: &[u8; SIZE]| *held == value;
        self.slot_found(frame, (queue, work), (word_hash(&value), &same), &|| value)
    }

    /// As [`UniformSlots::slot`], for the value of `head` followed by
    /// `rows`' bytes. The table is hashed when it is made
    /// ([`RowOffsets`]), so a draw hashes `head` alone and builds the
    /// value only to write it.
    ///
    /// # Panics
    ///
    /// As [`UniformSlots::slot`], or when `head` and the table do not
    /// fill `SIZE` bytes.
    pub fn slot_with_rows<const HEAD: usize>(
        &mut self,
        frame: &FrameEncoder,
        (queue, work): (&wgpu::Queue, &WorkCounter),
        (head, rows): ([u8; HEAD], &RowOffsets),
    ) -> Option<u32> {
        assert_eq!(
            HEAD + RowOffsets::CAPACITY * 8,
            SIZE,
            "{}: a {HEAD}-byte head and the rows' offsets do not fill a {SIZE}-byte value \
             (uniform_slots.rs)",
            self.label
        );
        let hash = word_hash(&head).rotate_left(5) ^ rows.hash();
        let same = |held: &[u8; SIZE]| held[..HEAD] == head && held[HEAD..] == rows.bytes()[..];
        let value = || {
            let mut value = [0; SIZE];
            value[..HEAD].copy_from_slice(&head);
            value[HEAD..].copy_from_slice(rows.bytes());
            value
        };
        self.slot_found(frame, (queue, work), (hash, &same), &value)
    }

    /// The slot of the value of hash `hash` that `same` accepts, else a
    /// slot `value` is written into.
    fn slot_found(
        &mut self,
        frame: &FrameEncoder,
        (queue, work): (&wgpu::Queue, &WorkCounter),
        (hash, same): (u64, &Same<'_, SIZE>),
        value: &dyn Fn() -> [u8; SIZE],
    ) -> Option<u32> {
        self.turn.begin(frame);
        let held = self.by_hash.get(&hash).and_then(|slots| {
            let mut held = slots.iter().copied();
            held.find(|&at| same(&self.slots[at].value))
        });
        let at = match held {
            Some(at) => at,
            None => self.write(frame, (queue, work), (hash, value()))?,
        };
        self.slots[at].encoder = frame.id();
        Some(self.offset(at))
    }

    fn write(
        &mut self,
        frame: &FrameEncoder,
        (queue, work): (&wgpu::Queue, &WorkCounter),
        (hash, value): (u64, [u8; SIZE]),
    ) -> Option<usize> {
        let at = self.free_slot(frame)?;
        work.write_uniform(queue, &self.buffer, u64::from(self.offset(at)), &value);
        let slot = Slot {
            value,
            hash,
            encoder: frame.id(),
        };
        let was = std::mem::replace(&mut self.slots[at], slot);
        if let Some(slots) = self.by_hash.get_mut(&was.hash) {
            slots.retain(|&held| held != at);
        }
        self.by_hash.entry(hash).or_default().push(at);
        Some(at)
    }

    /// A slot no draw of `frame` reads: an unused one in the buffer,
    /// else the one longest unread (its encoder closed before `frame`
    /// began), else the first of a buffer twice the size.
    fn free_slot(&mut self, frame: &FrameEncoder) -> Option<usize> {
        if self.slots.len() < self.capacity() {
            return Some(self.push(frame));
        }
        let stale = (0..self.slots.len())
            .filter(|&at| self.slots[at].encoder < frame.id())
            .min_by_key(|&at| self.slots[at].encoder);
        if stale.is_some() {
            return stale;
        }
        self.grow(frame)?;
        Some(self.push(frame))
    }

    fn push(&mut self, frame: &FrameEncoder) -> usize {
        self.slots.push(Slot {
            value: [0; SIZE],
            hash: 0,
            encoder: frame.id(),
        });
        self.slots.len() - 1
    }

    /// Moves to a buffer twice the size, leaving every slot behind:
    /// draws already recorded keep the old buffer through their bind
    /// group. Refused when the size is over `max_buffer_size`.
    fn grow(&mut self, frame: &FrameEncoder) -> Option<()> {
        let limit = frame.device().limits().max_buffer_size;
        let size = self.buffer.size() * 2;
        if size > limit {
            frame.refuse(ArenaFull {
                pipeline: self.label,
                size,
                limit,
                holding: Holding::OneFrame,
            });
            return None;
        }
        self.buffer = buffer(frame.device(), self.label, size);
        self.slots.clear();
        self.by_hash.clear();
        self.generation += 1;
        Some(())
    }

    fn capacity(&self) -> usize {
        usize::try_from(self.buffer.size() / self.align).expect("a buffer the device allows")
    }

    fn offset(&self, at: usize) -> u32 {
        u32::try_from(at as u64 * self.align).expect("an offset in a buffer the device allows")
    }
}

/// Hashes a uniform value eight bytes at a time (an `FxHash`-style
/// multiply and rotate): a value is up to a kilobyte, looked up on every
/// draw, and holds no input an attacker chooses. Zero bytes hash to 0.
pub(crate) fn word_hash(bytes: &[u8]) -> u64 {
    let mut hasher = WordHasher::default();
    std::hash::Hasher::write(&mut hasher, bytes);
    hasher.0
}

type WordHashing = std::hash::BuildHasherDefault<WordHasher>;

#[derive(Default)]
struct WordHasher(u64);

impl std::hash::Hasher for WordHasher {
    fn write(&mut self, bytes: &[u8]) {
        let (words, rest) = bytes.as_chunks::<8>();
        for &word in words {
            self.add(u64::from_le_bytes(word));
        }
        for &byte in rest {
            self.add(u64::from(byte));
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

impl WordHasher {
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}

/// A new buffer's size: `FIRST_SLOTS` slots `align` apart, or as many
/// whole slots as the device's `max_buffer_size` holds, at least one;
/// growing past it is refused where it happens (`grow`).
fn first_size(device: &wgpu::Device, align: u64) -> u64 {
    let limit = device.limits().max_buffer_size;
    (FIRST_SLOTS * align).min(limit / align * align).max(align)
}

fn buffer(device: &wgpu::Device, label: &str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

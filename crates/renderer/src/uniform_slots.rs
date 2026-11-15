//! A pipeline's uniform values kept on the GPU between frames: each
//! distinct value (16 bytes, or `SIZE`) in its own slot of one buffer,
//! written when it first appears and bound after by its dynamic offset,
//! found by its bytes in a map. A frame whose values the buffer already
//! holds writes nothing; a bind group made for the buffer serves every
//! slot.

use crate::encoder_turn::EncoderTurn;
use crate::{ArenaFull, FrameEncoder, Holding, WorkCounter};

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
    /// The slot holding each value.
    by_value: std::collections::HashMap<[u8; SIZE], usize, WordHashing>,
    turn: EncoderTurn,
    /// Counts the buffers made; a bind group is for one buffer.
    generation: u64,
}

/// A slot's value and the last encoder whose draw read it.
struct Slot<const SIZE: usize> {
    value: [u8; SIZE],
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
            by_value: std::collections::HashMap::default(),
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
        self.turn.begin(frame);
        let at = match self.by_value.get(&value) {
            Some(&at) => at,
            None => self.write(frame, queue, work, value)?,
        };
        self.slots[at].encoder = frame.id();
        Some(self.offset(at))
    }

    fn write(
        &mut self,
        frame: &FrameEncoder,
        queue: &wgpu::Queue,
        work: &WorkCounter,
        value: [u8; SIZE],
    ) -> Option<usize> {
        let at = self.free_slot(frame)?;
        work.write_uniform(queue, &self.buffer, u64::from(self.offset(at)), &value);
        let was = std::mem::replace(&mut self.slots[at].value, value);
        if self.by_value.get(&was) == Some(&at) {
            self.by_value.remove(&was);
        }
        self.by_value.insert(value, at);
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
        self.by_value.clear();
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
/// draw, and holds no input an attacker chooses.
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

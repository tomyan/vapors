//! A table the shaders read (`docs/plans/flat-scene-f2.md`,
//! `flat-scene-f3.md`): one storage buffer of entries, each instance
//! naming the entries it reads. Values are written only when they
//! change; a frame that changes none writes nothing. The transform table
//! is a table of 4x4 matrices ([`TransformTable`]); the clip table one of
//! clip entries (`clip_table.rs`).

use std::sync::atomic::{AtomicU64, Ordering};

use crate::table_entries::{Entries, EntryValue};
use crate::transform_entries::Matrix;
use crate::{ArenaFull, FrameEncoder, WorkCounter};

/// Entries in a new table: the page, a few scroll frames and a batch's
/// rows, or a few clips. A frame needing more moves to a buffer twice as
/// large.
const FIRST_ENTRIES: u32 = 64;

/// The transform table: 4x4 matrices the vertex stage reads, each
/// instance naming its frame's entry and its row's entry in its batch's
/// range.
pub type TransformTable = Table<Matrix>;

/// The next table's id: each table's is its own, for a pipeline to tell
/// two tables apart (`instanced.rs`).
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// The table and the buffer holding it.
pub struct Table<V> {
    id: u64,
    label: &'static str,
    entries: Entries<V>,
    buffer: wgpu::Buffer,
    /// The buffer holds none of the values yet: the next write writes
    /// them all.
    fresh: bool,
    generation: u64,
    written: (u64, u64),
    /// Its own: its writes are counted by [`Table::take_written`], not in
    /// a pipeline's work.
    work: WorkCounter,
}

impl<V: EntryValue> Table<V> {
    /// A table of entry 0 alone, for the pipelines labelled `label`.
    #[must_use]
    pub fn new(device: &wgpu::Device, label: &'static str) -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            label,
            entries: Entries::new(FIRST_ENTRIES),
            buffer: buffer::<V>(device, label, FIRST_ENTRIES),
            fresh: true,
            generation: 1,
            written: (0, 0),
            work: WorkCounter::default(),
        }
    }

    /// The first of `count` entries taken, the table grown when no free
    /// range holds them; `None` when the grown buffer would be over the
    /// device's limits (the refusal is kept on `frame`).
    pub fn take(&mut self, frame: &FrameEncoder, count: u32) -> Option<u32> {
        if let Some(at) = self.entries.take(count) {
            return Some(at);
        }
        self.grow(frame, count)?;
        self.entries.take(count)
    }

    /// Gives back the `count` entries from `at`; a table that poisons
    /// writes them with the poison before its next draw
    /// ([`Table::prepare`]).
    pub fn give(&mut self, at: u32, count: u32) {
        self.entries.give(at, count);
    }

    /// The value entry `at` holds.
    pub(crate) fn get(&self, at: u32) -> V {
        self.entries.get(at)
    }

    /// Sets the entries from `at` to `values`, writing those that
    /// changed for the draws of `frame`.
    ///
    /// # Panics
    ///
    /// When an entry already written for `frame` would change
    /// (`Entries::set`).
    pub(crate) fn set_values(
        &mut self,
        frame: &FrameEncoder,
        queue: &wgpu::Queue,
        (at, values): (u32, &[V]),
    ) {
        let changed = self.entries.set(at, values, frame.id());
        if self.fresh {
            self.prepare(queue);
            return;
        }
        if let Some(span) = changed {
            self.write(queue, span);
        }
        self.write_poison(queue);
    }

    /// Writes every value when the buffer holds none yet, and the poison
    /// of entries given back: before a draw that binds the table.
    pub fn prepare(&mut self, queue: &wgpu::Queue) {
        if self.fresh {
            self.fresh = false;
            let _ = self.entries.take_poisoned();
            self.write(queue, (0, self.entries.capacity() - 1));
        }
        self.write_poison(queue);
    }

    fn write_poison(&mut self, queue: &wgpu::Queue) {
        for span in self.entries.take_poisoned() {
            self.write(queue, span);
        }
    }

    /// As [`Table::prepare`], the write counted in `work` as
    /// well: for a pipeline's own identity table, whose writes no painter
    /// takes (`instanced.rs`).
    pub(crate) fn prepare_counted(&mut self, queue: &wgpu::Queue, work: &WorkCounter) {
        if self.fresh {
            self.prepare(queue);
            let (_, bytes) = self.take_written();
            work.count_buffer_bytes(bytes);
        }
    }

    /// This table's own id, unlike any other table's.
    #[must_use]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The whole buffer, for a bind group.
    #[must_use]
    pub fn binding(&self) -> wgpu::BindingResource<'_> {
        self.buffer.as_entire_binding()
    }

    /// Which buffer [`Table::binding`] names.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The entries and bytes written since last asked.
    pub fn take_written(&mut self) -> (u64, u64) {
        std::mem::take(&mut self.written)
    }

    /// How many entries' values changed since last asked, a given-back
    /// entry's poison included: unlike [`Table::take_written`], not the
    /// whole buffer a new one writes.
    pub fn take_changed(&mut self) -> u64 {
        self.entries.take_changed()
    }

    fn write(&mut self, queue: &wgpu::Queue, (first, last): (u32, u32)) {
        let bytes = self.entries.bytes((first, last));
        let at = u64::from(first) * V::BYTES;
        self.work.write_buffer(queue, &self.buffer, at, &bytes);
        self.written.0 += u64::from(last - first + 1);
        self.written.1 += bytes.len() as u64;
    }

    /// Moves to a buffer at least twice as large with room for `count`
    /// more; draws already recorded keep the old one through their bind
    /// group. Refused over `max_buffer_size` or
    /// `max_storage_buffer_binding_size`.
    fn grow(&mut self, frame: &FrameEncoder, count: u32) -> Option<()> {
        let (limit, name) = applied_limit(&frame.device().limits());
        let capacity = (self.entries.capacity() * 2).max(self.entries.capacity() + count);
        let size = u64::from(capacity) * V::BYTES;
        if size > limit {
            frame.refuse(ArenaFull {
                pipeline: self.label,
                size,
                limit,
                holding: V::holding(name),
            });
            return None;
        }
        self.entries.grow(capacity);
        self.buffer = buffer::<V>(frame.device(), self.label, capacity);
        (self.fresh, self.generation) = (true, self.generation + 1);
        Some(())
    }
}

/// The lower of the two limits on the table's buffer, and its name.
fn applied_limit(limits: &wgpu::Limits) -> (u64, &'static str) {
    if limits.max_storage_buffer_binding_size < limits.max_buffer_size {
        let size = limits.max_storage_buffer_binding_size;
        (size, "max_storage_buffer_binding_size")
    } else {
        (limits.max_buffer_size, "max_buffer_size")
    }
}

fn buffer<V: EntryValue>(device: &wgpu::Device, label: &str, entries: u32) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: u64::from(entries) * V::BYTES,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod tests;

impl TransformTable {
    /// Sets the entries from `at` to `values`, writing those that
    /// changed for the draws of `frame`.
    ///
    /// # Panics
    ///
    /// When an entry already written for `frame` would change
    /// (`table_entries.rs` `Entries::set`).
    pub fn set(&mut self, frame: &FrameEncoder, queue: &wgpu::Queue, at: u32, values: &[Matrix]) {
        self.set_values(frame, queue, (at, values));
    }

    /// The matrix entry `at` holds.
    #[must_use]
    pub fn matrix(&self, at: u32) -> Matrix {
        self.get(at)
    }
}

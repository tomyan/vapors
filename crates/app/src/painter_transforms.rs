//! The painter's transform table (`docs/plans/flat-scene-f2.md`): an
//! entry per frame its batches are drawn in (the page, a scroll frame, a
//! framed lane's template), holding the frame's shift, and a range per
//! batch over a lane's rows, holding each row's offset from the lane's
//! template. Each is kept while drawn and given back after a frame that
//! does not draw it, as kept bytes are (`painter_kept.rs`), so a frame's
//! entry, which its batches' kept bytes name, does not move under them.

use std::collections::HashMap;

use vapors_elements::{BatchFrame, BatchKey};
use vapors_renderer::{Shift, Target, TransformTable, translation};

/// The table, and what each frame and lane batch holds in it.
pub(super) struct Transforms {
    pub(super) table: TransformTable,
    frames: HashMap<BatchFrame, Entry>,
    rows: HashMap<BatchKey, Entry>,
    /// Counts the frames: an entry drawn in this one has it as `drawn`.
    frame: u64,
}

/// Entries held: the first, how many, and the frame that last drew them.
struct Entry {
    at: u32,
    count: u32,
    drawn: u64,
}

impl Transforms {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        Self {
            table: TransformTable::new(device, "painter transforms"),
            frames: HashMap::new(),
            rows: HashMap::new(),
            frame: 0,
        }
    }

    /// Starts a frame: nothing drawn yet.
    pub(super) fn begin_frame(&mut self) {
        self.frame += 1;
    }

    /// The entry of `frame`, set to `shift`; `None` when the table could
    /// not grow (the refusal is on the target's encoder).
    pub(super) fn frame_entry(
        &mut self,
        target: &Target<'_>,
        frame: BatchFrame,
        shift: Shift,
    ) -> Option<u32> {
        let at = held(
            &mut self.table,
            target,
            (&mut self.frames, frame, 1),
            self.frame,
        )?;
        let value = translation(shift.pixels());
        self.table.set(target.encoder, target.queue, at, &[value]);
        Some(at)
    }

    /// The first entry of batch `key`'s rows, each set to its offset in
    /// `offsets`; 0, the identity, for a batch in one frame (no offsets).
    /// `None` when the table could not grow.
    pub(super) fn rows_base(
        &mut self,
        target: &Target<'_>,
        key: BatchKey,
        offsets: &[[f32; 2]],
    ) -> Option<u32> {
        if offsets.is_empty() {
            return Some(0);
        }
        let count = u32::try_from(offsets.len()).expect("a batch's rows, at most 128");
        let at = held(
            &mut self.table,
            target,
            (&mut self.rows, key, count),
            self.frame,
        )?;
        let values: Vec<_> = offsets.iter().map(|&offset| translation(offset)).collect();
        self.table.set(target.encoder, target.queue, at, &values);
        Some(at)
    }

    /// Gives back the entries of the frames and batches this frame did
    /// not draw.
    pub(super) fn release_undrawn(&mut self) {
        let frame = self.frame;
        for held in [&mut self.frames as &mut dyn Release, &mut self.rows] {
            held.release(frame, &mut self.table);
        }
    }
}

/// The entries `key` holds in `held`, `count` of them, marked drawn in
/// `frame`: those held when they are as many, else taken anew.
fn held<K: std::hash::Hash + Eq>(
    table: &mut TransformTable,
    target: &Target<'_>,
    (held, key, count): (&mut HashMap<K, Entry>, K, u32),
    frame: u64,
) -> Option<u32> {
    if let Some(entry) = held.get_mut(&key).filter(|entry| entry.count == count) {
        entry.drawn = frame;
        return Some(entry.at);
    }
    if let Some(old) = held.remove(&key) {
        table.give(old.at, old.count);
    }
    let at = table.take(target.encoder, count)?;
    held.insert(
        key,
        Entry {
            at,
            count,
            drawn: frame,
        },
    );
    Some(at)
}

/// Entries by key that a frame not drawing them gives back.
trait Release {
    fn release(&mut self, frame: u64, table: &mut TransformTable);
}

impl<K: std::hash::Hash + Eq> Release for HashMap<K, Entry> {
    fn release(&mut self, frame: u64, table: &mut TransformTable) {
        for (_, entry) in self.extract_if(|_, entry| entry.drawn != frame) {
            table.give(entry.at, entry.count);
        }
    }
}

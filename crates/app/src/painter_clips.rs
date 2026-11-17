//! The painter's clip table (flat-scene F3.1, `docs/plans/flat-scene-f3.md`):
//! an entry per clip its quad, shadow and shape batches are drawn under,
//! keyed by the batch's frame and the clip's rectangle in that frame
//! (device pixels, as the scene gives it), holding the rectangle and the
//! frame's entry in the transform table. Set every frame it is drawn, so
//! a frame entry that moved is followed (item 4); kept while drawn and
//! given back after a frame that does not draw it, before the frame
//! entries are (item 2's order). F3.1 joins no batches across clips yet,
//! so a batch draws under one entry; text and image batches keep the
//! scissor until F3.1b moves them onto the table, which retires the
//! agreement check below.

use std::collections::HashMap;

use vapors_elements::BatchFrame;
use vapors_renderer::{ClipEntry, ClipTable, Target};
use vapors_scene::Rect;

/// The table, and the entry each frame's clip holds in it.
pub(super) struct Clips {
    pub(super) table: ClipTable,
    held: HashMap<(BatchFrame, [u32; 4]), Held>,
    /// Counts the frames: an entry drawn in this one has it as `drawn`.
    frame: u64,
}

/// An entry held: where, the frame entry it names, and the frame that
/// last drew it.
struct Held {
    at: u32,
    frame_entry: u32,
    drawn: u64,
}

/// A rectangle's key: its edges to the bit.
fn bits(rect: Rect) -> [u32; 4] {
    [rect.left, rect.top, rect.right, rect.bottom].map(f32::to_bits)
}

impl Clips {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        Self {
            table: ClipTable::new(device, "painter clips"),
            held: HashMap::new(),
            frame: 0,
        }
    }

    /// Starts a frame: nothing drawn yet.
    pub(super) fn begin_frame(&mut self) {
        self.frame += 1;
    }

    /// The entry of `clip`, in `frame` whose transform entry is
    /// `frame_entry`, set to them for this frame; `None` when the table
    /// could not grow (the refusal is on the target's encoder).
    pub(super) fn entry(
        &mut self,
        target: &Target<'_>,
        (frame, frame_entry): (BatchFrame, u32),
        clip: Rect,
    ) -> Option<u32> {
        let key = (frame, bits(clip));
        let at = match self.held.get(&key) {
            Some(held) => held.at,
            None => self.table.take(target.encoder, 1)?,
        };
        let rect = [clip.left, clip.top, clip.right, clip.bottom];
        self.table.set(
            target.encoder,
            target.queue,
            at,
            ClipEntry::rect(rect, frame_entry),
        );
        let drawn = self.frame;
        self.held.insert(
            key,
            Held {
                at,
                frame_entry,
                drawn,
            },
        );
        Some(at)
    }

    /// Gives back the entries this frame did not draw, writing their
    /// poison now (no draw of this frame reads them, and the write lands
    /// before the next frame's draws), so an idle frame writes nothing;
    /// their indices.
    pub(super) fn release_undrawn(&mut self, queue: &wgpu::Queue) -> Vec<u32> {
        let frame = self.frame;
        let undrawn: Vec<u32> = self
            .held
            .extract_if(|_, held| held.drawn != frame)
            .map(|(_, held)| held.at)
            .collect();
        for &at in &undrawn {
            self.table.give(at, 1);
        }
        self.table.prepare(queue);
        undrawn
    }

    /// Panics, naming the entry, when a held clip names one of the frame
    /// entries `released`: a frame entry is held while a held clip names
    /// it (item 4), so the clip is never read in a freed frame.
    pub(super) fn assert_none_names(&self, released: &[u32]) {
        let named = self
            .held
            .values()
            .find(|held| released.contains(&held.frame_entry));
        if let Some(held) = named {
            panic!(
                "clip entry {} names frame entry {}, given back while the clip is held: clips are \
                 given back before frames, and a frame entry is held while a held clip names it \
                 (painter_clips.rs, flat-scene-f3.md item 4); reproduce with `cargo xtask test`",
                held.at, held.frame_entry
            );
        }
    }

    /// In a debug build, panics when a batch drawn under the scissor
    /// (text, images: F3.1b moves them) and the clip entry the instanced
    /// batches of its frame use for the same clip disagree on where the
    /// clip is on the page: `on_page`, the scissor's rectangle, against
    /// the entry's rectangle moved by `shift` and cut to `frame_clip`.
    #[cfg(debug_assertions)]
    pub(super) fn assert_agrees(
        &self,
        (frame, clip): (BatchFrame, Rect),
        (shift, frame_clip): ([f32; 2], Option<Rect>),
        on_page: Option<Rect>,
    ) {
        let Some(held) = self.held.get(&(frame, bits(clip))) else {
            return;
        };
        let [left, top, right, bottom] = self.table.entry(held.at).rect;
        let entry = Rect::new(left, top, right, bottom);
        // as FrameDraw::on_page moves it: not at all by a zero shift
        let moved = if shift == [0.0, 0.0] {
            entry
        } else {
            entry.offset(shift)
        };
        let entry_on_page = frame_clip.map_or(moved, |outer| moved.intersection(outer));
        assert_eq!(
            Some(entry_on_page),
            on_page,
            "a scissor-path batch's clip and clip entry {} disagree on the page (painter_clips.rs; \
             F3.1b retires the scissor path); reproduce with `cargo xtask test`",
            held.at
        );
    }
}

#[cfg(test)]
#[path = "painter_clips_tests.rs"]
mod tests;

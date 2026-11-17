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
//! agreement check below. A clip's edges are whole device pixels until
//! F4a (`assert_whole`).

use std::collections::HashMap;

use vapors_elements::{BatchFrame, FrameDraw};
use vapors_renderer::{ClipEntry, ClipTable, Target, TransformTable};
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
    ///
    /// # Panics
    ///
    /// When an edge of `clip` is not a whole device pixel
    /// ([`assert_whole`]).
    pub(super) fn entry(
        &mut self,
        target: &Target<'_>,
        (frame, frame_entry): (BatchFrame, u32),
        clip: Rect,
    ) -> Option<u32> {
        assert_whole(clip);
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
        self.hold(key, at, frame_entry);
        Some(at)
    }

    /// `at` held for `key`, naming `frame_entry`, drawn this frame.
    fn hold(&mut self, key: (BatchFrame, [u32; 4]), at: u32, frame_entry: u32) {
        let drawn = self.frame;
        self.held.insert(
            key,
            Held {
                at,
                frame_entry,
                drawn,
            },
        );
    }

    /// Gives back the entries this frame did not draw, writing their
    /// poison now, so an idle frame writes nothing; their indices. The
    /// write lands at this frame's submit, before this frame's draws run:
    /// safe because no draw of this frame names an entry it did not draw.
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
}

/// Panics, naming the clip, when an edge of `clip` is not a whole device
/// pixel. The clip shader's one-pixel ramp (`clip.wgsl`) covers a pixel
/// exactly as the scissor (`Rect::scissor`, which rounds each edge) does
/// only at whole edges; paint snaps every clip under `WholeTranslate`
/// (`vapors-elements` `paint.rs`). F4a, which brings clips that are not
/// whole and a scale-aware edge, retires this check.
pub(super) fn assert_whole(clip: Rect) {
    let edges = [clip.left, clip.top, clip.right, clip.bottom];
    assert!(
        edges.iter().all(|edge| edge.fract() == 0.0),
        "clip {clip:?} has an edge that is not a whole device pixel: the clip table's one-pixel \
         edge equals the scissor only at whole edges (painter_clips.rs, clip.wgsl); snap the \
         clip as paint does, or build F4a's scale-aware edge; reproduce with `cargo xtask test`"
    );
}

/// Panics when the clip shader (`shader_rect`) and the scissor path
/// would put a clip in different places on the page: the scissor path
/// (text and images until F3.1b) moves the batch's own `clip` by the
/// frame's shift and cuts it to the frame's clip (`FrameDraw::on_page`).
/// Every build: two table reads per clipped batch drawn. F3.1b retires
/// it with the scissor path.
pub(super) fn assert_agrees(
    (transforms, clips): (&TransformTable, &ClipTable),
    at: u32,
    (clip, draw): (Rect, FrameDraw),
) {
    let entry = clips.entry(at);
    let shader = shader_rect(transforms, &entry, draw.clip);
    assert_eq!(
        Some(shader),
        draw.on_page(Some(clip)),
        "clip entry {at} (frame entry {}) is on the page elsewhere than the scissor path puts \
         the batch's clip {clip:?} (shader first): the frame entry's translation and the \
         frame's shift {:?} differ (painter_clips.rs, painter_transforms.rs); reproduce with \
         `cargo xtask test`",
        entry.frame,
        draw.shift
    );
}

/// Where the clip shader puts `entry` on the page: its rectangle moved
/// by the translation its frame entry holds in the transform table, cut
/// by the instanced batch's scissor to the frame's clip `frame_clip`.
fn shader_rect(transforms: &TransformTable, entry: &ClipEntry, frame_clip: Option<Rect>) -> Rect {
    let matrix = transforms.matrix(entry.frame);
    let [left, top, right, bottom] = entry.rect;
    let moved = Rect::new(left, top, right, bottom).offset([matrix[12], matrix[13]]);
    frame_clip.map_or(moved, |outer| moved.intersection(outer))
}

#[cfg(test)]
#[path = "painter_clips_tests.rs"]
mod tests;

//! The painter's clip table (flat-scene F3.1, `docs/plans/flat-scene-f3.md`):
//! an entry per clip its quad, shadow, shape, text and icon batches are
//! drawn under, holding the rectangle, its radii, the frame's entry in the
//! transform table and the entry it is under. An item clip is held by its
//! frame, edges, radii and parent; a frame's clip by its frame alone, on
//! the page under transform entry 0, every item entry of the frame under
//! it (`docs/plans/frame-clips.md`, slice 1). Set every frame it is drawn,
//! so an entry whose frame entry moved follows (item 4); kept while drawn
//! and given back after a frame that does not draw it, before the frame
//! entries are (item 2's order), and never while a held entry is under
//! it. Image, surface and mesh batches keep the scissor until F3.1b moves
//! them onto the table, which retires the agreement check below. A clip's
//! edges are whole device pixels until F4a (`assert_whole`).

use std::collections::HashMap;

use vapors_elements::{BatchFrame, FrameDraw};
use vapors_renderer::{ClipEntry, ClipTable, Target, TransformTable};
use vapors_scene::{Clip, Rect};

/// The table, and the entry each frame's clip holds in it.
pub(super) struct Clips {
    pub(super) table: ClipTable,
    held: HashMap<Key, Held>,
    /// Counts the frames: an entry drawn in this one has it as `drawn`.
    frame: u64,
    /// The scratch a batch's clip entries are worked out in, reused batch
    /// after batch so a frame allocates none (`painter_kept_entries.rs`).
    pub(super) runs: Vec<(u32, u32)>,
}

/// What an entry is held by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Key {
    /// An item clip: its frame, its edges and radii, and the entry it is
    /// under.
    Item(BatchFrame, [u32; 8], u32),
    /// A frame's clip, by the frame alone: when an outer frame scrolls,
    /// the clip moves on the page and its entry is set again in place,
    /// so the entries under it, and the kept bytes naming them, stay
    /// (vapors-perf `gpu_f3_baselines`).
    Frame(BatchFrame),
}

/// An entry held: where, the frame entry it names, and the frame that
/// last drew it.
struct Held {
    at: u32,
    frame_entry: u32,
    drawn: u64,
}

/// A clip's key: its edges and corner radii to the bit.
fn bits(clip: Clip) -> [u32; 8] {
    let Rect {
        left,
        top,
        right,
        bottom,
    } = clip.rect();
    let [a, b, c, d] = clip.radii();
    [left, top, right, bottom, a, b, c, d].map(f32::to_bits)
}

impl Clips {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        Self {
            table: ClipTable::new(device, "painter clips"),
            held: HashMap::new(),
            frame: 0,
            runs: Vec::new(),
        }
    }

    /// Starts a frame: nothing drawn yet.
    pub(super) fn begin_frame(&mut self) {
        self.frame += 1;
    }

    /// The entry of `clip`, in `frame` whose transform entry is
    /// `frame_entry`, under entry `parent` (its frame's clip, 0 for
    /// none), set to them for this frame; `None` when the table could not
    /// grow (the refusal is on the target's encoder).
    ///
    /// # Panics
    ///
    /// When an edge of `clip` is not a whole device pixel
    /// ([`assert_whole`]).
    pub(super) fn entry(
        &mut self,
        target: &Target<'_>,
        (frame, frame_entry): (BatchFrame, u32),
        (clip, parent): (Clip, u32),
    ) -> Option<u32> {
        assert_whole(clip.rect());
        assert!(
            clip.outer().is_none(),
            "clip {clip:?} reached the clip table cut to an outer rectangle: a batch's clips are \
             in its frame, whose own clip is the scissor (painter_runs.rs), and only a clip put on \
             the page carries one (vapors-scene clip.rs `cut`); reproduce with `cargo xtask test`"
        );
        let entry = ClipEntry::rounded(edges(clip.rect()), clip.radii(), frame_entry).under(parent);
        self.held_entry(target, Key::Item(frame, bits(clip), parent), entry)
    }

    /// The entry of `frame`'s clip `rect`, on the page (device pixels,
    /// transform entry 0, the identity): what every entry of the frame's
    /// items on the table is under, and the entry of its items with no
    /// clip of their own (`docs/plans/frame-clips.md`, slice 1). Held by
    /// the frame, set to `rect` each frame drawn: a frame's own scroll
    /// leaves its clip where it is; an outer frame's moves it, in place.
    /// `None` when the table could not grow.
    ///
    /// # Panics
    ///
    /// When an edge of `rect` is not a whole device pixel.
    pub(super) fn frame_clip(
        &mut self,
        target: &Target<'_>,
        (frame, rect): (BatchFrame, Rect),
    ) -> Option<u32> {
        assert_whole(rect);
        let entry = ClipEntry::rounded(edges(rect), [0.0; 4], 0);
        self.held_entry(target, Key::Frame(frame), entry)
    }

    /// The entry held by `key`, taken when it is not, set to `entry` for
    /// this frame.
    fn held_entry(&mut self, target: &Target<'_>, key: Key, entry: ClipEntry) -> Option<u32> {
        let at = match self.held.get(&key) {
            Some(held) => held.at,
            None => self.table.take(target.encoder, 1)?,
        };
        self.table.set(target.encoder, target.queue, at, entry);
        self.hold(key, at, entry.frame);
        Some(at)
    }

    /// `at` held for `key`, naming `frame_entry`, drawn this frame.
    fn hold(&mut self, key: Key, at: u32, frame_entry: u32) {
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
        self.assert_no_parent_in(&undrawn);
        for &at in &undrawn {
            self.table.give(at, 1);
        }
        self.table.prepare(queue);
        undrawn
    }

    /// Panics, naming both, when a held entry is under one of `released`:
    /// a frame's entry is drawn with every entry under it
    /// (`painter_kept_entries.rs` `table_entries`), so it is held while
    /// they are, and the shader never walks to a freed entry.
    fn assert_no_parent_in(&self, released: &[u32]) {
        let orphan = self.held.iter().find_map(|(key, held)| match key {
            Key::Item(_, _, parent) if released.contains(parent) => Some((held.at, *parent)),
            _ => None,
        });
        if let Some((at, parent)) = orphan {
            panic!(
                "clip entry {at} is under clip entry {parent}, given back while it is held: a \
                 frame's clip entry is drawn with every entry under it (painter_kept_entries.rs \
                 `table_entries`, docs/plans/frame-clips.md); reproduce with `cargo xtask test`"
            );
        }
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

/// `rect`'s edges as a clip entry holds them.
fn edges(rect: Rect) -> [f32; 4] {
    [rect.left, rect.top, rect.right, rect.bottom]
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
/// (images, surfaces and meshes until F3.1b's last part) moves the
/// batch's own `clip` by the
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

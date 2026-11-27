//! The table entries a kept batch's bytes name (flat-scene F2, F3.1):
//! its frame's in the transform table, its clip's in the clip table, and
//! its lane part's rows'.

use vapors_renderer::Target;
use vapors_scene::Run;

use crate::painter::ScenePainter;
use crate::painter::rows::Moved;
use crate::painter::runs::table_clip;

/// A run's frame entry, its items' clip entries as runs `(first item,
/// entry)` (the painter's scratch, `Clips::runs`, to give back), and its
/// rows' first entry.
pub(super) type TableEntries = (u32, Vec<(u32, u32)>, u32);

impl ScenePainter {
    /// A run's frame's entry in the transform table, set to its shift;
    /// its items' clip entries as runs `(first item, entry)`, each under
    /// its frame's clip entry, which an item with no clip takes (entry 0
    /// for a frame with no clip, and for images, surfaces and meshes,
    /// still under the scissor: `painter_clips.rs`); and the first of its lane part's rows'
    /// entries (0 for a batch in one frame). `None` when a table could not
    /// grow (the refusal is on the target's encoder).
    pub(super) fn table_entries(
        &mut self,
        target: &Target<'_>,
        run: &Run,
        moved: &Moved<'_>,
    ) -> Option<TableEntries> {
        let frame = self
            .transforms
            .frame_entry(target, moved.frame, moved.shift)?;
        // a run on the table is cut by its frame's clip there too
        let frame_clip = moved.draw.clip.filter(|_| run.clipped_by_table());
        let parent = match frame_clip {
            Some(rect) => self.frame_entries(target, (moved.frame, rect), moved.draw.rounded)?,
            None => 0,
        };
        // the scratch, reused batch after batch: no allocation a batch
        let mut clips = std::mem::take(&mut self.clips.runs);
        clips.clear();
        for &(first, clip) in moved.clips.runs() {
            let entry = match table_clip(run, clip) {
                Some(clip) => self.clip_entry(target, (frame, clip, parent), moved)?,
                None => parent,
            };
            clips.push((first, entry));
        }
        let rows_base = self
            .transforms
            .rows_base(target, moved.rows_key, moved.offsets)?;
        Some((frame, clips, rows_base))
    }

    /// The entry a frame's item entries are under: its clip `rect`'s, or
    /// its rounded clip's when it has one, that under the square one.
    fn frame_entries(
        &mut self,
        target: &Target<'_>,
        (frame, rect): (vapors_elements::BatchFrame, vapors_scene::Rect),
        rounded: Option<vapors_scene::Clip>,
    ) -> Option<u32> {
        let square = self.clips.frame_clip(target, (frame, rect))?;
        match rounded {
            Some(round) => self.clips.frame_round(target, (frame, round, square)),
            None => Some(square),
        }
    }

    /// The entry of `clip` in `moved`'s frame, whose transform entry is
    /// `frame`, under entry `parent`, checked against the scissor path's
    /// placing of it.
    fn clip_entry(
        &mut self,
        target: &Target<'_>,
        (frame, clip, parent): (u32, vapors_scene::Clip, u32),
        moved: &Moved<'_>,
    ) -> Option<u32> {
        let at = self
            .clips
            .entry(target, (moved.frame, frame), (clip, parent))?;
        let tables = (&self.transforms.table, &self.clips.table);
        crate::painter::clips::assert_agrees(tables, at, (clip.rect(), moved.draw));
        Some(at)
    }
}

#[cfg(test)]
#[path = "painter_frame_clips_tests.rs"]
mod frame_clips_tests;

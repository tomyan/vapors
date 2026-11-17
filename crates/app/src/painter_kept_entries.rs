//! The table entries a kept batch's bytes name (flat-scene F2, F3.1):
//! its frame's in the transform table, its clip's in the clip table, and
//! its lane part's rows'.

use vapors_renderer::Target;
use vapors_scene::Run;

use super::Kind;
use crate::painter::ScenePainter;
use crate::painter::rows::Moved;

impl ScenePainter {
    /// For the runs placed through the transform table (all but meshes):
    /// their frame's entry, set to its shift, their clip's entry (0 for
    /// none, and for the runs still under the scissor until F3.1b:
    /// `painter_clips.rs`), and the first of their lane part's rows'
    /// entries (0 for a batch in one frame); zeros for meshes. `None` when
    /// a table could not grow (the refusal is on the target's encoder).
    pub(super) fn table_entries(
        &mut self,
        target: &Target<'_>,
        run: &Run,
        moved: &Moved<'_>,
    ) -> Option<(u32, u32, u32)> {
        if matches!(run, Run::Meshes(_)) {
            return Some((0, 0, 0));
        }
        let frame = self
            .transforms
            .frame_entry(target, moved.frame, moved.shift)?;
        let on_the_table = matches!(
            Kind::of_run(run),
            Some(Kind::Shadows | Kind::Quads | Kind::ClosedForms)
        );
        let clip = match moved.clip.filter(|_| on_the_table) {
            Some(clip) => self.clip_entry(target, (frame, clip), moved)?,
            None => 0,
        };
        let rows_base = self
            .transforms
            .rows_base(target, moved.rows_key, moved.offsets)?;
        Some((frame, clip, rows_base))
    }

    /// The entry of `clip` in `moved`'s frame, whose transform entry is
    /// `frame`, checked against the scissor path's placing of it.
    fn clip_entry(
        &mut self,
        target: &Target<'_>,
        (frame, clip): (u32, vapors_scene::Rect),
        moved: &Moved<'_>,
    ) -> Option<u32> {
        let at = self.clips.entry(target, (moved.frame, frame), clip)?;
        let tables = (&self.transforms.table, &self.clips.table);
        crate::painter::clips::assert_agrees(tables, at, (clip, moved.draw));
        Some(at)
    }
}

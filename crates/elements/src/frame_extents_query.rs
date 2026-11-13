//! Reading the extents index (`frame_extents.rs`): the steps of a
//! frame that can show in a view, and a step's item count.

use vapors_scene::Rect;

use super::{Entry, Extents};

impl Extents {
    /// How many items the step at `place` paints; 0 when it draws
    /// nothing.
    pub(crate) fn items_at(&self, place: usize) -> u64 {
        let Some((frame, top, tall)) = self.at[place] else {
            return 0;
        };
        let index = &self.frames[&frame];
        if tall {
            index.tall[index.tall_position(place)].items
        } else {
            index.entries[index.position(top, place)].items
        }
    }

    /// The entries of frame `frame` whose extent meets `view` (frame
    /// coordinates) top to bottom: every step that can show in it.
    ///
    /// # Panics
    ///
    /// When `view`'s top or bottom is not a number: testing every item
    /// would show all of them, and the index would show none.
    pub(crate) fn candidates(&self, frame: usize, view: Rect) -> impl Iterator<Item = &Entry> {
        assert!(
            !view.top.is_nan() && !view.bottom.is_nan(),
            "scroll frame {frame} is asked for the steps in view {view:?}, whose edges are not \
             numbers: a frame's shift or clip is not a number (frames.rs, scroll::frame_shift); \
             reproduce with `cargo xtask test`"
        );
        let (entries, tall) = self.frames.get(&frame).map_or((&[][..], &[][..]), |index| {
            (&index.entries[..], &index.tall[..])
        });
        let first = entries.partition_point(|entry| entry.reach <= view.top);
        let last = entries.partition_point(|entry| entry.top < view.bottom);
        let between = entries.get(first..last).unwrap_or_default();
        let meets = move |entry: &&Entry| entry.bottom > view.top && entry.top < view.bottom;
        tall.iter().chain(between).filter(meets)
    }

    /// How many entries [`Extents::candidates`] tests for `view`.
    #[cfg(test)]
    pub(crate) fn visited(&self, frame: usize, view: Rect) -> usize {
        let index = &self.frames[&frame];
        let first = index
            .entries
            .partition_point(|entry| entry.reach <= view.top);
        let last = index
            .entries
            .partition_point(|entry| entry.top < view.bottom);
        index.tall.len() + last.saturating_sub(first)
    }
}

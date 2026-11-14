//! Reading the extents index (`frame_extents.rs`): the steps of a
//! frame that can show in a view, and a step's item count.

use vapors_scene::Rect;

use super::{Entry, Extents, FrameIndex, missing};

impl Extents {
    /// The frames with a drawing step.
    #[cfg(test)]
    pub(crate) fn frames(&self) -> impl Iterator<Item = usize> + '_ {
        self.frames.keys().copied()
    }

    /// The frames holding a step that spans every view.
    pub(crate) fn spanning_frames(&self) -> impl Iterator<Item = usize> + '_ {
        self.spanning.keys().copied()
    }

    /// How many items every drawing step paints.
    pub(crate) fn items(&self) -> u64 {
        self.frames.values().map(|index| index.items).sum()
    }

    /// How many items the step at `place` paints; 0 when it draws
    /// nothing.
    pub(crate) fn items_at(&self, place: usize) -> u64 {
        let Some((frame, top, tall)) = self.at[place] else {
            return 0;
        };
        let index = &self.frames[&frame];
        if tall {
            return index.tall[index.tall_position(place)].items;
        }
        let entry = index.entries.get(index.position(top, place));
        entry.map_or_else(|| missing(place), |entry| entry.items)
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
        let index = self.frames.get(&frame);
        let tall = index.map_or(&[][..], |index| &index.tall[..]);
        let short = index.map(|index| index.in_reach(view));
        let meets = move |entry: &&Entry| entry.bottom > view.top && entry.top < view.bottom;
        tall.iter().chain(short.into_iter().flatten()).filter(meets)
    }

    /// How many entries [`Extents::candidates`] tests for `view`.
    #[cfg(test)]
    pub(crate) fn visited(&self, frame: usize, view: Rect) -> usize {
        let index = &self.frames[&frame];
        index.tall.len() + index.in_reach(view).count()
    }
}

impl FrameIndex {
    /// The short entries from the first whose reach passes `view`'s top
    /// to the last whose top is before its bottom.
    fn in_reach(&self, view: Rect) -> impl Iterator<Item = &Entry> {
        let first = self
            .entries
            .partition_point(|entry| entry.reach <= view.top);
        self.entries
            .iter_from(first)
            .take_while(move |entry| entry.top < view.bottom)
    }
}

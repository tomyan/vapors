//! The clips of a batch's items (flat-scene F3.1, step 2,
//! `docs/plans/flat-scene-f3.md` item 10): runs of consecutive items
//! under one clip, each its first item and the clip. A batch of a kind
//! whose clip is applied by the scissor has one run, as each clip change
//! starts a new batch; a batch of a kind that takes its clip from the
//! clip table ([`crate::Run::clipped_by_table`]) joins items across clip
//! changes, each run naming its own entry.

use crate::clip::Clip;

/// A batch's items' clips, as runs `(first item, clip)`; never empty,
/// the first run at item 0.
#[derive(Clone, Debug, PartialEq)]
pub struct Clips(Vec<(u32, Option<Clip>)>);

impl Clips {
    /// Every item under `clip`.
    #[must_use]
    pub fn one(clip: Option<Clip>) -> Self {
        Self(vec![(0, clip)])
    }

    /// Item `at`, the next, under `clip`: a new run when it differs from
    /// the last item's.
    ///
    /// # Panics
    ///
    /// When `at` is not past the last run's first item: the runs must
    /// rise, each item reading the last run at or before it.
    pub fn push(&mut self, at: u32, clip: Option<Clip>) {
        let last_first = self.0.last().map_or(0, |&(first, _)| first);
        assert!(
            at > last_first,
            "clip runs {:?} given item {at}: items are pushed in order, after the last run's \
             first (batch_clips.rs)",
            self.0
        );
        if self.last() != clip {
            self.0.push((at, clip));
        }
    }

    /// The last item's clip.
    #[must_use]
    pub fn last(&self) -> Option<Clip> {
        self.0.last().and_then(|&(_, clip)| clip)
    }

    /// The one clip every item has; `None` when the items have more than
    /// one.
    #[must_use]
    pub fn uniform(&self) -> Option<Option<Clip>> {
        match self.0.as_slice() {
            [(_, clip)] => Some(*clip),
            _ => None,
        }
    }

    /// Item `at`'s clip.
    #[must_use]
    pub fn at(&self, at: usize) -> Option<Clip> {
        let after = self.0.partition_point(|&(first, _)| first as usize <= at);
        self.0[after.saturating_sub(1)].1
    }

    /// The smallest rectangle holding every item's clip: unclipped
    /// (`None`) when any item is.
    #[must_use]
    pub fn union(&self) -> Option<crate::display::Rect> {
        let mut clips = self
            .0
            .iter()
            .map(|&(_, clip)| clip.map(|clip| clip.bounds()));
        let first = clips.next().flatten()?;
        clips.try_fold(first, |union, clip| clip.map(|clip| union.union(clip)))
    }

    /// The runs, `(first item, clip)`.
    #[must_use]
    pub fn runs(&self) -> &[(u32, Option<Clip>)] {
        &self.0
    }
}

#[cfg(test)]
#[path = "batch_clips_tests.rs"]
mod tests;

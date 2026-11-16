//! Each kept batch's key (IU4, `docs/plans/iu4.md` section 2): what a
//! painter keeps the batch's GPU bytes under. A batch made again keeps
//! the key of the earlier batch it is matched with when their runs are
//! equal, so its bytes are neither built nor written again; any other
//! batch gets a new key. Runs are equal when they are the same to the
//! bit (`run_bits.rs`), as the bytes kept for them are: `==` would take
//! -0.0 for 0.0; and, for a batch over a lane's rows (4f), when each
//! item is in the same row, as the bytes hold each item's row. Batches
//! are matched by place from the start up to the
//! first place whose runs differ, and after it by place moved by one
//! offset, so a batch put in or taken out (a background added, say)
//! leaves the batches after it their keys, and a scroll, which drops
//! batches at one edge of the view and adds them at the other, leaves
//! the batches between their keys. Equal runs draw equal bytes, so a
//! match is never wrong, only missed.

use std::sync::atomic::{AtomicU64, Ordering};

use vapors_scene::Batched;

use crate::framed_batches::{BatchRows, FramedBatched};
use crate::frames_draw::BatchFrame;
use crate::run_bits::same_bits;

/// How many of the new batches after the common prefix are looked for
/// among the old ones: a scroll by one row or a row put in drops or adds
/// a few batches before the first that stayed.
const OFFSET_SEARCH: usize = 8;

/// The next key handed out, on any thread: no two batches of any
/// document share one.
static NEXT: AtomicU64 = AtomicU64::new(1);

/// Names one batch's items while they stay the same: a batch whose
/// run changes, or that is made for another view, has another key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BatchKey(u64);

impl BatchKey {
    fn new() -> Self {
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// The batches of one view, each with its scroll frame and key, in the
/// same order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeptBatched {
    /// The batches, each in its frame's coordinates under its clip
    /// inside the frame (`framed_batches.rs`).
    pub batched: Batched,
    /// Each batch's frame, by its place in `batched.batches`.
    pub frames: Vec<BatchFrame>,
    /// Each batch's rows, by its place: empty for a batch in one frame
    /// (`framed_batches.rs`).
    pub rows: Vec<BatchRows>,
    /// Each batch's key, by its place in `batched.batches`.
    pub keys: Vec<BatchKey>,
}

impl KeptBatched {
    /// `batched`, in `frames`, with keys: those of `was`'s batches
    /// matched with equal runs, else new ones. A run's bytes are in its
    /// frame's coordinates, so a match in another frame is drawn as is.
    pub(crate) fn keyed(framed: FramedBatched, was: Option<&Self>) -> Self {
        let keys = match was {
            Some(was) => was.keys_for(&framed),
            None => framed
                .batched
                .batches
                .iter()
                .map(|_| BatchKey::new())
                .collect(),
        };
        Self {
            batched: framed.batched,
            frames: framed.frames,
            rows: framed.rows,
            keys,
        }
    }

    /// Whether `self`'s batch at `was` draws the bytes `framed`'s at `now`
    /// does: the same run to the bit and each item in the same row.
    fn same(&self, was: usize, (framed, now): (&FramedBatched, usize)) -> bool {
        let (Some(old), Some(new)) = (
            self.batched.batches.get(was),
            framed.batched.batches.get(now),
        ) else {
            return false;
        };
        same_bits(&old.run, &new.run) && self.rows.get(was) == framed.rows.get(now)
    }

    /// A key for each of `batched`'s batches: `self`'s at the same place
    /// before the first place whose runs differ; after it, `self`'s at
    /// the place moved by the offset (`offset_after`) when the runs are
    /// equal, else at the same place counted from the end when the runs
    /// are equal and the offset matched no other batch with it; else a
    /// new one.
    fn keys_for(&self, framed: &FramedBatched) -> Vec<BatchKey> {
        let (was, now) = (&self.batched.batches, &framed.batched.batches);
        let shared = was.len().min(now.len());
        let prefix = (0..shared)
            .take_while(|&at| self.same(at, (framed, at)))
            .count();
        let offset = self.offset_after(framed, prefix);
        let keys = (0..now.len()).map(|at| {
            if at < prefix {
                self.keys[at]
            } else {
                let moved = self.key_moved(framed, (at, prefix), offset);
                let from_end = || self.key_from_end(framed, (at, prefix), offset);
                moved.or_else(from_end).unwrap_or_else(BatchKey::new)
            }
        });
        keys.collect()
    }

    /// How far `self`'s batches after `prefix` are moved from
    /// `batched`'s: from the first of `batched`'s batches there whose run
    /// one of `self`'s there has, to that one; else the difference in
    /// their counts, which matches them by place from the end. A scroll
    /// drops batches at one edge and adds them at the other; a batch put
    /// in or taken out moves those after it. Only the first
    /// `OFFSET_SEARCH` batches after the prefix are looked for, so the
    /// search is O(shown batches) when nothing matches (a reflow).
    fn offset_after(&self, framed: &FramedBatched, prefix: usize) -> isize {
        let (was, now) = (&self.batched.batches, &framed.batched.batches);
        let in_was = |at: usize| (prefix..was.len()).position(|old| self.same(old, (framed, at)));
        let after = prefix..now.len().max(prefix);
        let mut searched = after.take(OFFSET_SEARCH).enumerate();
        let first = searched.find_map(|(at, now)| Some((at, in_was(now)?)));
        first.map_or_else(
            || count(was.len()) - count(now.len()),
            |(now, was)| count(was) - count(now),
        )
    }

    /// `self`'s key at the place of `batched`'s `at`th moved by
    /// `offset`, when that place is not before `prefix` and the runs are
    /// equal.
    fn key_moved(
        &self,
        framed: &FramedBatched,
        (at, prefix): (usize, usize),
        offset: isize,
    ) -> Option<BatchKey> {
        let from = at.checked_add_signed(offset)?;
        let matched = from >= prefix && self.same(from, (framed, at));
        matched.then(|| self.keys[from])
    }
}

impl KeptBatched {
    /// `self`'s key at the place of `batched`'s `at`th counted from the
    /// end, when that place is not before `prefix`, the runs are equal,
    /// and the batch `offset` maps to that place does not match it: each
    /// key is kept by one batch at most.
    fn key_from_end(
        &self,
        framed: &FramedBatched,
        (at, prefix): (usize, usize),
        offset: isize,
    ) -> Option<BatchKey> {
        let from_end = count(self.batched.batches.len()) - count(framed.batched.batches.len());
        let key = self.key_moved(framed, (at, prefix), from_end)?;
        let from = at.checked_add_signed(from_end)?;
        let claimant = from
            .checked_add_signed(-offset)
            .filter(|&other| other != at);
        let claimed = claimant.and_then(|other| self.key_moved(framed, (other, prefix), offset));
        claimed.is_none().then_some(key)
    }
}

/// `len` as a signed count: batch counts are far below `isize::MAX`.
fn count(len: usize) -> isize {
    isize::try_from(len).expect("fewer batches than isize::MAX")
}

#[cfg(test)]
#[path = "kept_batches_keys_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "kept_batches_keys_moved_tests.rs"]
mod moved_tests;

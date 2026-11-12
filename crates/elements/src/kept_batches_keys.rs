//! Each kept batch's key (IU4, `docs/plans/iu4.md` section 2): what a
//! painter keeps the batch's GPU bytes under. A batch made again keeps
//! the key of the earlier batch it is matched with when their runs are
//! equal, so its bytes are neither built nor written again; any other
//! batch gets a new key. Batches are matched by place from the start up
//! to the first place whose runs differ, and by place from the end
//! after it, so a batch put in or taken out (a background added, say)
//! leaves the batches after it their keys. Equal runs draw equal bytes,
//! so a match is never wrong, only missed.

use std::sync::atomic::{AtomicU64, Ordering};

use vapors_scene::Batched;

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

/// The batches of one view and each one's key, in the same order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeptBatched {
    /// The batches, as [`vapors_scene::batched`] makes them.
    pub batched: Batched,
    /// Each batch's key, by its place in `batched.batches`.
    pub keys: Vec<BatchKey>,
}

impl KeptBatched {
    /// `batched` with keys: those of `was`'s batches matched with equal
    /// runs, else new ones.
    pub(crate) fn keyed(batched: Batched, was: Option<&Self>) -> Self {
        let keys = match was {
            Some(was) => was.keys_for(&batched),
            None => batched.batches.iter().map(|_| BatchKey::new()).collect(),
        };
        Self { batched, keys }
    }

    /// A key for each of `batched`'s batches: `self`'s at the same place
    /// before the first place whose runs differ; after it, `self`'s at
    /// the same place counted from the end when the runs are equal;
    /// else a new one.
    fn keys_for(&self, batched: &Batched) -> Vec<BatchKey> {
        let (was, now) = (&self.batched.batches, &batched.batches);
        let same = was
            .iter()
            .zip(now)
            .take_while(|(was, now)| was.run == now.run);
        let prefix = same.count();
        let keys = (0..now.len()).map(|at| {
            if at < prefix {
                self.keys[at]
            } else {
                let key = self.key_from_end(batched, (at, prefix));
                key.unwrap_or_else(BatchKey::new)
            }
        });
        keys.collect()
    }

    /// `self`'s key at the place of `batched`'s `at`th counted from the
    /// end, when that place is not before `prefix` and the runs are equal.
    fn key_from_end(&self, batched: &Batched, (at, prefix): (usize, usize)) -> Option<BatchKey> {
        let (was, now) = (&self.batched.batches, &batched.batches);
        let from_end = (at + was.len()).checked_sub(now.len())?;
        let matched = from_end >= prefix && was.get(from_end)?.run == now[at].run;
        matched.then(|| self.keys[from_end])
    }
}

#[cfg(test)]
#[path = "kept_batches_keys_tests.rs"]
mod tests;

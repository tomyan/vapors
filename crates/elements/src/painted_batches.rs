//! `Painted::batches`: the kept batches of the last draw, read when the
//! view is the same and made again otherwise (`kept_batches.rs`).

use std::cell::Ref;

use vapors_scene::{Batched, Rect};

use crate::kept_batches::KeptBatches;
use crate::kept_batches_keys::KeptBatched;
use crate::painted::Painted;

impl Painted {
    /// The scene's batches for a target whose drawable area is `view`
    /// (device pixels): [`Painted::kept_batches`] without their keys.
    ///
    /// # Panics
    /// As [`Painted::kept_batches`].
    #[must_use]
    pub fn batches(&self, view: Rect) -> Ref<'_, Batched> {
        Ref::map(self.kept_batches(view), |kept| &kept.batched)
    }

    /// The scene's batches for a target whose drawable area is `view`
    /// (device pixels), each with its key: kept from the last draw when
    /// its view was `view` and no step it showed was painted again
    /// since; made again otherwise, a batch keeping its key when the
    /// batch at its place had an equal run (`kept_batches_keys.rs`).
    ///
    /// # Panics
    /// In a debug build, when they differ from batching the whole scene,
    /// naming the first differing batch. Always, when called while a
    /// `Ref` from an earlier call is held and the batches must be made.
    #[must_use]
    pub fn kept_batches(&self, view: Rect) -> Ref<'_, KeptBatched> {
        let kept = Ref::filter_map(self.kept_batches.borrow(), |kept| {
            kept.as_ref()
                .filter(|kept| kept.view == view)
                .and_then(KeptBatches::current)
        })
        .unwrap_or_else(|held| {
            drop(held);
            self.make_batches(view)
        });
        #[cfg(debug_assertions)]
        crate::kept_batches_check::assert_same(&kept.batched, &self.scene().batches(view));
        kept
    }

    /// The kept batches made for `view`, then read.
    fn make_batches(&self, view: Rect) -> Ref<'_, KeptBatched> {
        let mut slot = self.kept_batches.try_borrow_mut().unwrap_or_else(|_| {
            panic!(
                "Painted::batches needs to make the batches while a Ref from an earlier call is \
                 held (painted_batches.rs); drop the Ref before drawing at another view or after an \
                 update; reproduce with `cargo xtask test`"
            )
        });
        let drawn = (&self.drawn, self.order.steps.len());
        KeptBatches::for_view(&mut slot, view, drawn, &self.kept_boxes);
        drop(slot);
        Ref::map(self.kept_batches.borrow(), |kept| {
            kept.as_ref()
                .and_then(KeptBatches::current)
                .expect("made above")
        })
    }
}

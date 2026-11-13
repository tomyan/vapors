//! `Painted::batches`: the kept batches of the last draw, read when the
//! view is the same and made again otherwise (`kept_batches.rs`).

use std::cell::Ref;

use vapors_scene::{Batched, Rect};

use crate::kept_batches::{KeptBatches, Sources};
use crate::kept_batches_keys::KeptBatched;
use crate::painted::Painted;

impl Painted {
    /// The scene's batches for a target whose drawable area is `view`
    /// (device pixels), each in its scroll frame's coordinates:
    /// [`Painted::kept_batches`] without their frames and keys.
    ///
    /// # Panics
    /// As [`Painted::kept_batches`].
    #[must_use]
    pub fn batches(&self, view: Rect) -> Ref<'_, Batched> {
        Ref::map(self.kept_batches(view), |kept| &kept.batched)
    }

    /// The scene's batches for a target whose drawable area is `view`
    /// (device pixels), each in its scroll frame's coordinates under its
    /// clip inside the frame, with its frame ([`Painted::frame_draw`]
    /// gives the shift and clip to draw it with) and its key: kept from
    /// the last draw when its view was `view` and no step it showed was
    /// painted again since; made again otherwise, a batch keeping its
    /// key when an earlier batch had an equal run (`kept_batches_keys.rs`).
    ///
    /// # Panics
    /// In a debug build, when they differ from batching the whole scene
    /// in its frames, or their items put on the page from batching it on
    /// the page, naming the first difference. Always, when called while a
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
        self.assert_batches_hold(&kept, view);
        kept
    }

    /// Panics when `kept` differs from batching the scene afresh in its
    /// frames, or its items put on the page from batching it on the page
    /// (`kept_batches_check.rs`).
    #[cfg(debug_assertions)]
    fn assert_batches_hold(&self, kept: &KeptBatched, view: Rect) {
        let scene = self.scene();
        crate::kept_batches_check::assert_same(kept, &scene.framed_batches(view));
        let page = scene.batches(view);
        crate::kept_batches_check::assert_on_page(kept, &self.frames, &page);
    }

    /// What the kept batches are sorted and made from.
    pub(crate) fn batch_sources(&self) -> Sources<'_> {
        Sources {
            drawn: &self.drawn,
            extents: &self.extents,
            boxes: &self.kept_boxes,
            frames: &self.frames,
        }
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
        KeptBatches::for_view(&mut slot, view, self.batch_sources());
        drop(slot);
        Ref::map(self.kept_batches.borrow(), |kept| {
            kept.as_ref()
                .and_then(KeptBatches::current)
                .expect("made above")
        })
    }
}

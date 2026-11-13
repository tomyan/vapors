//! The off-path workloads (`--off-path`): pages outside the shapes the
//! kept paths are built and judged on (the rows page and the racer),
//! timed and counted like them so their costs show as numbers. They
//! report; `--scaling` never judges them. Each is a [`UnitPage`] of
//! one [`Kind`] and a change made before each frame.
//!
//! The 10,000-unit Insert frames of the scroller kinds take seconds of
//! CPU each, so a run at the default counts takes tens of minutes:
//! choose with `--only` and give small counts.

use std::rc::Rc;

use vapors_app::{ElementsApp, FrameTiming};
use vapors_platform::{App, Next};

pub use crate::off_path_kinds::{Kind, off_path_workloads};
use crate::off_path_moved::animate;
use crate::off_path_page::{UnitPage, page_viewport};
use crate::run::{Counts, Workload};

/// The line an `--off-path` run prints after its setup, so its output
/// names itself wherever it is pasted.
pub const OFF_PATH_SETUP: &str = "Workloads: off-path, reported only (--scaling does not judge \
     them); the 10,000-unit Insert frames take seconds each, so choose with --only and small counts";

/// The pixels a scroll change moves its scroller by each frame.
const SCROLL_STEP: f32 = 24.0;

/// What changes before each frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OffPathChange {
    /// The middle unit's value, to a new five-digit text. The middle
    /// unit is out of view at every size, as the rows page's Text edit
    /// is: an off-screen edit.
    Edit,
    /// The body scrolled down 24 px; back to the top at the end.
    Scroll,
    /// The second unit's own scroller scrolled right 24 px; back to its
    /// start at the end. Only the x axis is driven, though a code block
    /// can scroll down too.
    ScrollInner,
    /// Alternating frames: a unit inserted before the middle unit, then
    /// that unit removed. The report has a section for each kind of frame
    /// (`frame_kinds.rs`).
    Insert,
    /// The container's offset moved to the next of eight fractional
    /// offsets (`off_path_moved.rs`): the whole subtree moves every frame.
    Animate,
}

/// An off-path page in an app, and its change.
pub struct OffPath {
    page: UnitPage,
    app: ElementsApp,
    change: OffPathChange,
}

impl OffPath {
    /// `count` units of `kind`, updated once, in an app that has drawn
    /// no frame and times its frames on the GPU.
    ///
    /// # Panics
    /// The page is refused (a CSS property vapors does not honour, which
    /// the unit tests' mount of every kind catches first), or `kind`
    /// does not offer `change`.
    #[must_use]
    pub fn new(kind: Kind, count: usize, change: OffPathChange) -> Self {
        assert!(
            kind.changes().contains(&change),
            "{kind:?} does not offer {change:?}; it offers {:?} (off_path_kinds.rs)",
            kind.changes()
        );
        let page = UnitPage::new(&kind.shape(), count).unwrap_or_else(|error| {
            panic!(
                "{kind:?} at {count} is refused: {error} (cargo test -p vapors-perf --lib off_path)"
            )
        });
        let mut app = ElementsApp::new(Rc::clone(&page.doc));
        app.time_frames(true);
        app.mirror_a11y(true).expect("one mirror per document");
        Self { page, app, change }
    }
}

impl Workload for OffPath {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 640 x 400 at 1x
    fn size(&self) -> (u32, u32) {
        let viewport = page_viewport();
        (viewport.width as u32, viewport.height as u32)
    }

    fn change(&mut self, sample: usize) {
        // the consumer's part, as the rows workload takes it
        let _taken = self.app.ax_update();
        match self.change {
            OffPathChange::Edit => self.page.edit(sample),
            OffPathChange::Scroll => self.page.scroll_body(SCROLL_STEP),
            OffPathChange::ScrollInner => self.page.scroll_inner(SCROLL_STEP),
            OffPathChange::Insert => self.page.toggle_inserted(),
            OffPathChange::Animate => animate(&self.page, sample),
        }
    }

    fn app(&mut self) -> &mut dyn App {
        &mut self.app
    }

    fn check(&self, next: Next) -> Result<(), String> {
        match (next, self.app.failure().get()) {
            (_, Some(failure)) => Err(failure),
            (Next::Exit, None) => Err("the app exited without a failure".into()),
            _ => Ok(()),
        }
    }

    fn counts(&self) -> Counts {
        let frame = self.app.frame_counts();
        Counts::of(&self.page.doc.borrow(), frame, self.app.render_passes())
    }

    fn timing(&self) -> FrameTiming {
        self.app.frame_timing()
    }

    fn frame_kinds(&self) -> &'static [&'static str] {
        match self.change {
            OffPathChange::Insert => &["insert", "remove"],
            // Animate too: each step moves the subtree by a fraction of a pixel, the same work
            OffPathChange::Edit
            | OffPathChange::Scroll
            | OffPathChange::ScrollInner
            | OffPathChange::Animate => &["every"],
        }
    }
}

#[cfg(test)]
#[path = "off_path_tests.rs"]
mod tests;

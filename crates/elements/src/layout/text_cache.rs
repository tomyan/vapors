//! Parley layouts kept between updates (F2c S7). A run laid out at a
//! width is kept by its text, its styles and that width; measuring and
//! painting ask for it, and only a run that changed is laid out again.
//! A start-aligned run at a width its max-content layout fits in takes
//! that layout: its lines and glyphs would be the same. Justify is
//! left out deliberately, to be conservative. Runs with a negative or
//! NaN letter spacing are left out too. A reused layout differs from a
//! fresh one in parley's `layout_max_advance` (`f32::MAX`) and in each
//! line's `inline_max_coord` (the content width); nothing in vapors
//! reads either, but a reader of those fields would see the difference.
//! Reuse compares widths in device pixels, so at a fractional scale
//! rounding may stop it firing: a performance limit, not a correctness
//! one.
//! An entry records the update that last asked for it. A sweep drops
//! those older than the last update whose layout nothing else holds (a
//! kept paint holds what it shows); it runs when the map has doubled
//! since the last sweep, so its cost is amortised O(1) per insert.

use rustc_hash::FxHashMap;
use std::rc::Rc;

use vapors_text::{Align, FontSystem, Layout};

use super::inline::InlineRun;
use super::measure::text_layout;
pub(crate) use super::text_key::Key;
use crate::style::TextStyle;

/// The layouts kept, each with the update that last asked for it.
#[derive(Clone, Default)]
pub(crate) struct TextCache {
    entries: FxHashMap<Key, Entry>,
    /// The update under way, counting from 1 (`begin`).
    update: u64,
    /// How many entries the last sweep left.
    swept_to: usize,
    built: usize,
}

#[derive(Clone)]
struct Entry {
    layout: Rc<Layout>,
    asked: u64,
}

impl TextCache {
    /// `run` laid out at `width` CSS pixels at `scale`: kept, or laid
    /// out now.
    pub(crate) fn layout(
        &mut self,
        fonts: &mut FontSystem,
        scale: f32,
        run: &InlineRun,
        width: Option<f32>,
    ) -> Rc<Layout> {
        let key = Key::of(run, width, scale);
        if let Some(layout) = self.asked(&key) {
            return layout;
        }
        let layout = self.unwrapped(run, width, scale).unwrap_or_else(|| {
            self.built += 1;
            Rc::new(text_layout(fonts, scale, run, width))
        });
        self.insert(key, Rc::clone(&layout));
        layout
    }

    /// `run`'s max-content layout, kept, when laying it out at `width`
    /// would give the same: the run is start-aligned, so its lines do
    /// not move within the width, and no line is wider than `width`, so
    /// none breaks. The layout's `layout_max_advance` and each line's
    /// `inline_max_coord` are those of the max-content layout, not of
    /// one built at `width`.
    fn unwrapped(&mut self, run: &InlineRun, width: Option<f32>, scale: f32) -> Option<Rc<Layout>> {
        let width = width?;
        if !reusable(run) {
            return None;
        }
        let layout = self.asked(&Key::of(run, None, scale))?;
        fits(layout.width(), width * scale).then_some(layout)
    }

    /// The layout kept by `key`, recorded as asked for this update.
    fn asked(&mut self, key: &Key) -> Option<Rc<Layout>> {
        let entry = self.entries.get_mut(key)?;
        entry.asked = self.update;
        Some(Rc::clone(&entry.layout))
    }

    /// Keeps `layout` by `key`, sweeping first when the map has doubled
    /// since the last sweep.
    fn insert(&mut self, key: Key, layout: Rc<Layout>) {
        if self.entries.len() >= 2 * self.swept_to.max(1) {
            self.sweep();
        }
        let asked = self.update;
        self.entries.insert(key, Entry { layout, asked });
    }

    /// Drops the entries asked for before the last update whose layout
    /// nothing else holds.
    fn sweep(&mut self) {
        let since = self.update.saturating_sub(1);
        self.entries
            .retain(|_, entry| entry.asked >= since || Rc::strong_count(&entry.layout) > 1);
        self.swept_to = self.entries.len();
    }

    /// Starts an update.
    pub(crate) fn begin(&mut self) {
        self.update += 1;
        self.built = 0;
    }

    /// How many layouts this update built rather than kept.
    pub(crate) fn built(&self) -> usize {
        self.built
    }
}

/// Whether a max-content layout of `run` can stand in for one at a
/// width: it is start-aligned and no letter spacing, its own or a
/// span's, is negative or NaN (a negative one can make the running x
/// non-monotonic, so a position mid-line can pass the final width).
fn reusable(run: &InlineRun) -> bool {
    let spacing = |style: &TextStyle| style.letter_spacing >= 0.0;
    matches!(run.style.align, Align::Start)
        && spacing(&run.style)
        && run.spans.iter().all(|span| spacing(&span.style))
}

/// Whether a line `content` wide fits `available`. A NaN or infinite
/// number on either side does not fit, so it is laid out, not reused.
fn fits(content: f32, available: f32) -> bool {
    content.is_finite() && available.is_finite() && content <= available
}

#[cfg(test)]
#[path = "text_cache_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "text_cache_age_tests.rs"]
mod age_tests;

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
//! kept paint holds what it shows).
//!
//! A sweep runs on an insert that finds the map doubled since the last
//! sweep, and at the start of an update when the map holds more than 4
//! times the live set: the entries the last update asked for or the
//! texts its paint showed, whichever is more (kept parts hold layouts
//! without asking, so asks alone undercount it). Bound: the map holds
//! at most twice what the last sweep left, and a document that shrinks
//! from 3000 runs to 10 drops the 3000 it no longer shows at the next
//! update's start.
//!
//! Cost: an insert sweep leaves `swept_to` entries and the next needs
//! `swept_to` more inserts, so O(1) amortised per insert. In the steady
//! state the map holds about the live set, under 4 times it, so no
//! update-start sweep runs and the check is O(1) per update. One that
//! runs keeps the last update's entries and what is held, which the
//! last paint shows, at most twice the live set; the next needs at
//! least twice the live set in new inserts first.

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
    /// How many entries this update asked for or inserted.
    asked_now: usize,
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
        if let Some(layout) = self
            .asked(&key)
            .or_else(|| self.unwrapped(run, width, scale))
        {
            return layout;
        }
        self.built += 1;
        let layout = Rc::new(text_layout(fonts, scale, run, width));
        self.insert(key, Rc::clone(&layout));
        layout
    }

    /// `run`'s max-content layout, kept, when laying it out at `width`
    /// would give the same. It is not kept again by the width: two map
    /// entries holding one layout would each count as a holder of the
    /// other, and a sweep would never drop either. The conditions: the run is start-aligned, so its lines do
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
        if entry.asked != self.update {
            entry.asked = self.update;
            self.asked_now += 1;
        }
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
        self.asked_now += 1;
    }

    /// Drops the entries asked for before the last update whose layout
    /// nothing else holds.
    fn sweep(&mut self) {
        let since = self.update.saturating_sub(1);
        self.entries
            .retain(|_, entry| entry.asked >= since || Rc::strong_count(&entry.layout) > 1);
        self.swept_to = self.entries.len();
    }

    /// Starts an update, the last one's paint showing `shown` texts;
    /// sweeps when the map holds more than 4 times the live set.
    pub(crate) fn begin(&mut self, shown: usize) {
        let live = self.asked_now.max(shown).max(1);
        self.update += 1;
        self.built = 0;
        self.asked_now = 0;
        if self.entries.len() > 4 * live {
            self.sweep();
        }
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

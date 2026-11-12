//! Parley layouts kept between updates (F2c S7), in two maps. A run is
//! shaped once per text, styles, visibility, span positions and scale
//! (`text_key.rs`), whichever node holds it, so equal runs share one
//! shaping; each layout is that shaping broken at one width, kept by
//! the same and the width. Measuring and painting ask for layouts, and
//! only a run that changed is shaped or broken again. A hit needs the
//! whole key equal, not only its hash.
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
//! those older than the last update that nothing else holds (a kept
//! paint holds the layouts it shows; nothing outside holds a shaping,
//! a layout holds its own copy). Both maps follow the rules below
//! (`text_kept.rs`), each counting its own entries, so the shapings
//! kept are bounded as the layouts are; every layout has one, so they
//! number at most the layouts the live set needs.
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

use std::rc::Rc;

use vapors_text::{Align, FontSystem, Layout, Shaped};

use super::inline::InlineRun;
#[cfg(test)]
use super::measure::text_layout;
use super::measure::text_shaped;
use super::text_kept::Kept;
pub(crate) use super::text_key::Key;
use super::text_key::ShapeKey;
use crate::style::TextStyle;

/// The layouts kept, and the shaped runs they were broken from.
#[derive(Clone, Default)]
pub(crate) struct TextCache {
    layouts: Kept<Key, Rc<Layout>>,
    shapes: Kept<ShapeKey, Rc<Shaped>>,
    built: usize,
    shaped: usize,
}

impl TextCache {
    /// `run` laid out at `width` CSS pixels at `scale`: kept, or broken
    /// now from its shaping, kept or done now.
    pub(crate) fn layout(
        &mut self,
        fonts: &mut FontSystem,
        scale: f32,
        run: &InlineRun,
        width: Option<f32>,
    ) -> Rc<Layout> {
        let key = Key::of(run, width, scale);
        if let Some(layout) = self.kept(&key).or_else(|| self.unwrapped(run, &key, width)) {
            return layout;
        }
        let shaped = self.shaped_run(fonts, scale, run, key.shape());
        self.built += 1;
        let layout = Rc::new(shaped.lay_out(width, run.style.align));
        self.layouts.insert(key, Rc::clone(&layout));
        layout
    }

    /// `run` shaped at `scale`: kept by `key`, or shaped now.
    fn shaped_run(
        &mut self,
        fonts: &mut FontSystem,
        scale: f32,
        run: &InlineRun,
        key: &ShapeKey,
    ) -> Rc<Shaped> {
        if let Some(shaped) = self.shapes.asked(key) {
            return Rc::clone(shaped);
        }
        self.shaped += 1;
        let shaped = Rc::new(text_shaped(fonts, scale, run));
        self.shapes.insert(key.clone(), Rc::clone(&shaped));
        shaped
    }

    /// `run`'s max-content layout, kept, when laying it out at `width`
    /// would give the same. It is not kept again by the width: two map
    /// entries holding one layout would each count as a holder of the
    /// other, and a sweep would never drop either. The conditions: the
    /// run is start-aligned, so its lines do not move within the width,
    /// and no line is wider than `width`, so none breaks. The layout's
    /// `layout_max_advance` and each line's `inline_max_coord` are those
    /// of the max-content layout, not of one built at `width`.
    fn unwrapped(&mut self, run: &InlineRun, key: &Key, width: Option<f32>) -> Option<Rc<Layout>> {
        let width = width?;
        if !reusable(run) {
            return None;
        }
        let layout = self.kept(&Key::at(key.shape().clone(), None))?;
        fits(layout.width(), width * layout.scale()).then_some(layout)
    }

    /// The layout kept by `key`, recorded as asked for this update.
    fn kept(&mut self, key: &Key) -> Option<Rc<Layout>> {
        self.layouts.asked(key).map(Rc::clone)
    }

    /// Starts an update, the last one's paint showing `shown` texts.
    pub(crate) fn begin(&mut self, shown: usize) {
        self.built = 0;
        self.shaped = 0;
        self.layouts.begin(shown);
        self.shapes.begin(shown);
    }

    /// How many layouts this update built rather than kept.
    pub(crate) fn built(&self) -> usize {
        self.built
    }

    /// How many runs this update shaped rather than kept.
    pub(crate) fn shaped(&self) -> usize {
        self.shaped
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

#[cfg(test)]
#[path = "text_cache_shape_tests.rs"]
mod shape_tests;

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
//! A layout no update asked for is dropped at the end of the next one.

use rustc_hash::FxHashMap;
use std::mem::{Discriminant, discriminant};
use std::rc::Rc;

use vapors_dom::NodeId;
use vapors_text::{Align, FontSystem, Layout, LineHeight};

use super::inline::{InlineRun, InlineSpan};
use super::measure::text_layout;
use crate::style::{Face, TextStyle};

/// What a layout is kept by: the run's text, styles and visibility,
/// and the width and scale, every float by its bits. Eq and Hash both
/// derive from these, so they agree (a NaN equals itself here).
#[derive(Clone, Hash, PartialEq, Eq)]
pub(crate) struct Key {
    text: String,
    style: StyleKey,
    visible: bool,
    spans: Vec<SpanKey>,
    width: Option<u32>,
    scale: u32,
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct SpanKey {
    node: NodeId,
    range: std::ops::Range<usize>,
    visible: bool,
    style: StyleKey,
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct StyleKey {
    face: Discriminant<Face>,
    /// Size, weight and letter spacing.
    numbers: [u32; 3],
    /// Which kind of line height, and its number.
    line_height: (u8, u32),
    color: [u32; 4],
    italic: bool,
    align: Discriminant<Align>,
}

impl Key {
    pub(crate) fn of(run: &InlineRun, width: Option<f32>, scale: f32) -> Self {
        Self {
            text: run.text.clone(),
            style: StyleKey::of(&run.style),
            visible: run.visible,
            spans: run.spans.iter().map(SpanKey::of).collect(),
            width: width.map(f32::to_bits),
            scale: scale.to_bits(),
        }
    }
}

impl SpanKey {
    fn of(span: &InlineSpan) -> Self {
        Self {
            node: span.node,
            range: span.range.clone(),
            visible: span.visible,
            style: StyleKey::of(&span.style),
        }
    }
}

impl StyleKey {
    fn of(style: &TextStyle) -> Self {
        let line_height = match style.line_height {
            LineHeight::Normal => (0, 0),
            LineHeight::FontSizeRelative(v) => (1, v.to_bits()),
            LineHeight::Px(v) => (2, v.to_bits()),
        };
        Self {
            face: discriminant(&style.face),
            numbers: [style.size, style.weight, style.letter_spacing].map(f32::to_bits),
            line_height,
            color: [style.color.r, style.color.g, style.color.b, style.color.a].map(f32::to_bits),
            italic: style.italic,
            align: discriminant(&style.align),
        }
    }
}

/// The layouts asked for this update, and those kept from the last.
#[derive(Clone, Default)]
pub(crate) struct TextCache {
    current: FxHashMap<Key, Rc<Layout>>,
    previous: FxHashMap<Key, Rc<Layout>>,
    built: usize,
}

impl TextCache {
    /// `run` laid out at `width` CSS pixels at `scale`: kept from this
    /// update or the last, or laid out now.
    pub(crate) fn layout(
        &mut self,
        fonts: &mut FontSystem,
        scale: f32,
        run: &InlineRun,
        width: Option<f32>,
    ) -> Rc<Layout> {
        let key = Key::of(run, width, scale);
        if let Some(layout) = self.current.get(&key) {
            return Rc::clone(layout);
        }
        let layout = self.previous.remove(&key);
        let layout = layout.or_else(|| self.unwrapped(run, width, scale));
        let layout = layout.unwrap_or_else(|| {
            self.built += 1;
            Rc::new(text_layout(fonts, scale, run, width))
        });
        self.current.insert(key, Rc::clone(&layout));
        layout
    }

    /// `run`'s max-content layout, kept, when laying it out at `width`
    /// would give the same: the run is start-aligned, so its lines do
    /// not move within the width, and no line is wider than `width`, so
    /// none breaks. It is kept for the next update too, which may ask
    /// only for widths. The layout's `layout_max_advance` and each
    /// line's `inline_max_coord` are those of the max-content layout,
    /// not of one built at `width`.
    fn unwrapped(&mut self, run: &InlineRun, width: Option<f32>, scale: f32) -> Option<Rc<Layout>> {
        let width = width?;
        if !reusable(run) {
            return None;
        }
        let key = Key::of(run, None, scale);
        let layout = self.current.get(&key).or_else(|| self.previous.get(&key))?;
        if !fits(layout.width(), width * scale) {
            return None;
        }
        let layout = Rc::clone(layout);
        self.current.insert(key, Rc::clone(&layout));
        Some(layout)
    }

    /// Starts an update: what the last one asked for is kept, what it
    /// did not is dropped.
    pub(crate) fn begin(&mut self) {
        self.previous = std::mem::take(&mut self.current);
        self.built = 0;
    }

    /// Keeps the layouts the last update used for the next, as when
    /// this update used them all: its paint is the last one's, kept.
    pub(crate) fn keep_previous(&mut self) {
        if self.current.is_empty() {
            std::mem::swap(&mut self.current, &mut self.previous);
            return;
        }
        for (key, layout) in self.previous.drain() {
            self.current.entry(key).or_insert(layout);
        }
    }

    /// Keeps the layout kept by `key` for the next update, when the last
    /// update asked for it: a kept paint shows it (`kept_box.rs`).
    pub(crate) fn touch(&mut self, key: &Key) {
        if let Some((key, layout)) = self.previous.remove_entry(key) {
            self.current.entry(key).or_insert(layout);
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

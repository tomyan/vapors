//! Parley layouts kept between updates (F2c S7). A run laid out at a
//! width is kept by its text, its styles and that width; measuring and
//! painting ask for it, and only a run that changed is laid out again.
//! A layout no update asked for is dropped at the end of the next one.

use std::collections::HashMap;
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
struct Key {
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
    fn of(run: &InlineRun, width: Option<f32>, scale: f32) -> Self {
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
#[derive(Default)]
pub(crate) struct TextCache {
    current: HashMap<Key, Rc<Layout>>,
    previous: HashMap<Key, Rc<Layout>>,
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
        let layout = self.previous.remove(&key).unwrap_or_else(|| {
            self.built += 1;
            Rc::new(text_layout(fonts, scale, run, width))
        });
        self.current.insert(key, Rc::clone(&layout));
        layout
    }

    /// Starts an update: what the last one asked for is kept, what it
    /// did not is dropped.
    pub(crate) fn begin(&mut self) {
        self.previous = std::mem::take(&mut self.current);
        self.built = 0;
    }

    /// How many layouts this update built rather than kept.
    pub(crate) fn built(&self) -> usize {
        self.built
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_whose_style_holds_a_nan_is_kept_like_any_other() {
        // Given: a letter spacing that is not a number
        let style = TextStyle {
            face: Face::Geist,
            size: 16.0,
            weight: 400.0,
            line_height: LineHeight::Normal,
            color: vapors_scene::Color::default(),
            letter_spacing: f32::NAN,
            align: Align::Start,
            italic: false,
        };
        let run = InlineRun {
            text: "nan".to_owned(),
            style,
            visible: true,
            spans: Vec::new(),
        };
        let (mut fonts, mut cache) = (FontSystem::bundle_only(), TextCache::default());

        // When: asked for twice
        cache.layout(&mut fonts, 1.0, &run, None);
        cache.layout(&mut fonts, 1.0, &run, None);

        // Then: laid out once
        assert_eq!(cache.built(), 1);
    }
}

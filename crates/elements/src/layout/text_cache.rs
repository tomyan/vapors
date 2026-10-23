//! Parley layouts kept between updates (F2c S7). A run laid out at a
//! width is kept by its text, its styles and that width; measuring and
//! painting ask for it, and only a run that changed is laid out again.
//! A layout no update asked for is dropped at the end of the next one.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use vapors_text::{FontSystem, Layout, LineHeight};

use super::inline::{InlineRun, InlineSpan};
use super::measure::text_layout;
use crate::style::TextStyle;

/// What a layout is kept by: the run and the width, the run compared
/// whole, so two runs never share a layout by a hash collision.
#[derive(Clone, PartialEq)]
struct Key {
    run: InlineRun,
    width: Option<u32>,
    scale: u32,
}

impl Eq for Key {}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.run.text.hash(state);
        self.width.hash(state);
        self.scale.hash(state);
        hash_style(&self.run.style, state);
        for span in &self.run.spans {
            hash_span(span, state);
        }
    }
}

fn hash_span<H: Hasher>(span: &InlineSpan, state: &mut H) {
    span.node.hash(state);
    span.range.hash(state);
    span.visible.hash(state);
    hash_style(&span.style, state);
}

fn hash_style<H: Hasher>(style: &TextStyle, state: &mut H) {
    std::mem::discriminant(&style.face).hash(state);
    for value in [style.size, style.weight, style.letter_spacing] {
        value.to_bits().hash(state);
    }
    match style.line_height {
        LineHeight::Normal => 0u32.hash(state),
        LineHeight::FontSizeRelative(v) | LineHeight::Px(v) => v.to_bits().hash(state),
    }
    for channel in [style.color.r, style.color.g, style.color.b, style.color.a] {
        channel.to_bits().hash(state);
    }
    (style.italic, std::mem::discriminant(&style.align)).hash(state);
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
        let key = Key {
            run: run.clone(),
            width: width.map(f32::to_bits),
            scale: scale.to_bits(),
        };
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

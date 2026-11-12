//! What the text cache (`text_cache.rs`) keeps a layout by.

use std::hash::{BuildHasher, Hash, Hasher};
use std::mem::{Discriminant, discriminant};

use rustc_hash::FxBuildHasher;
use vapors_dom::NodeId;
use vapors_text::{Align, LineHeight};

use super::inline::{InlineRun, InlineSpan};
use crate::style::{Face, TextStyle};

/// What a layout is kept by: the run's text, styles and visibility,
/// and the width and scale, every float by its bits, with their hash,
/// made once with the key so a lookup or `touch` does not hash the
/// text and spans again. Eq compares the hashes, then the parts.
#[derive(Clone)]
pub(crate) struct Key {
    hash: u64,
    parts: Parts,
}

/// A key's parts. Eq and Hash both derive from these, so they agree (a
/// NaN equals itself here).
#[derive(Clone, Hash, PartialEq, Eq)]
struct Parts {
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
        let parts = Parts::of(run, width, scale);
        Self {
            hash: FxBuildHasher.hash_one(&parts),
            parts,
        }
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.parts == other.parts
    }
}

impl Eq for Key {}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

impl Parts {
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

#[cfg(test)]
#[path = "text_key_tests.rs"]
mod tests;

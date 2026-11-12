//! What the text cache (`text_cache.rs`) keeps a layout by.

use std::hash::{BuildHasher, Hash, Hasher};
use std::mem::{Discriminant, discriminant};

use rustc_hash::FxBuildHasher;
use vapors_text::{Align, LineHeight};

use super::inline::{InlineRun, InlineSpan};
use crate::style::{Face, TextStyle};

/// What a value is kept by: its parts with their hash, made once with
/// the key so a lookup or `touch` does not hash the text and spans
/// again. Eq compares the hashes, then the parts.
#[derive(Clone)]
pub(crate) struct Hashed<P> {
    hash: u64,
    parts: P,
}

/// What a run is shaped by: its text, styles and visibility, span
/// positions relative to the run, and the scale, every float by its
/// bits. Not the node that holds it: shaping never reads it, so equal
/// runs in different nodes share one shaping.
pub(crate) type ShapeKey = Hashed<ShapeParts>;

/// What a layout is kept by: what its run is shaped by, and the width.
pub(crate) type Key = Hashed<(ShapeKey, Option<u32>)>;

/// A shape key's parts. Eq and Hash both derive from these, so they
/// agree (a NaN equals itself here).
#[derive(Clone, Hash, PartialEq, Eq)]
pub(crate) struct ShapeParts {
    text: String,
    style: StyleKey,
    visible: bool,
    spans: Vec<SpanKey>,
    scale: u32,
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct SpanKey {
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

impl<P: Hash> Hashed<P> {
    fn new(parts: P) -> Self {
        Self {
            hash: FxBuildHasher.hash_one(&parts),
            parts,
        }
    }
}

impl Key {
    pub(crate) fn of(run: &InlineRun, width: Option<f32>, scale: f32) -> Self {
        Self::at(ShapeKey::of(run, scale), width)
    }

    /// The layout of what `shape` keys, at `width`.
    pub(crate) fn at(shape: ShapeKey, width: Option<f32>) -> Self {
        Self::new((shape, width.map(f32::to_bits)))
    }

    /// What this layout's run is shaped by.
    pub(crate) fn shape(&self) -> &ShapeKey {
        &self.parts.0
    }
}

impl ShapeKey {
    pub(crate) fn of(run: &InlineRun, scale: f32) -> Self {
        Self::new(ShapeParts {
            text: run.text.clone(),
            style: StyleKey::of(&run.style),
            visible: run.visible,
            spans: run.spans.iter().map(SpanKey::of).collect(),
            scale: scale.to_bits(),
        })
    }
}

impl<P: PartialEq> PartialEq for Hashed<P> {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.parts == other.parts
    }
}

impl<P: Eq> Eq for Hashed<P> {}

impl<P> Hash for Hashed<P> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

impl SpanKey {
    fn of(span: &InlineSpan) -> Self {
        Self {
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

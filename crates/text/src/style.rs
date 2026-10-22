//! What a run of text looks like: family, size, weight, the OpenType
//! features the UI needs, and the ink colour.

use std::ops::Range;

use std::borrow::Cow;

use parley::setting::Tag;
use parley::{
    FontFamily, FontFeature, FontFeatures, FontVariation, FontVariations, LineHeight, StyleProperty,
};
use vapors_scene::Color;

use crate::fonts::{GEIST_FAMILY, GEIST_MONO_FAMILY};

/// Which family to shape with. Named families resolve through the font
/// system: the bundle first, then system fonts on native.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family<'a> {
    /// The bundled UI face.
    Geist,
    /// The bundled monospace.
    GeistMono,
    /// A family by name, as the font declares it.
    Named(&'a str),
}

impl<'a> Family<'a> {
    fn name(self) -> &'a str {
        match self {
            Self::Geist => GEIST_FAMILY,
            Self::GeistMono => GEIST_MONO_FAMILY,
            Self::Named(name) => name,
        }
    }
}

/// A text style. Build one with [`Style::new`] and override fields.
#[derive(Clone, Debug, PartialEq)]
pub struct Style<'a> {
    /// Font family.
    pub family: Family<'a>,
    /// Font size in logical pixels.
    pub size: f32,
    /// Weight on the `wght` axis, 100..900. Default 400.
    pub weight: f32,
    /// Optical size on the `opsz` axis; ignored by fonts without one.
    pub optical_size: Option<f32>,
    /// `tnum`: every figure takes the same advance. Default off.
    pub tabular_figures: bool,
    /// `kern`. Default on.
    pub kerning: bool,
    /// Line height as a multiple of the font size. Default 1.2.
    pub line_height: f32,
    /// Ink colour. Default opaque black.
    pub color: Color,
}

impl<'a> Style<'a> {
    /// The defaults above at `size` in `family`.
    #[must_use]
    pub fn new(family: Family<'a>, size: f32) -> Self {
        Self {
            family,
            size,
            weight: 400.0,
            optical_size: None,
            tabular_figures: false,
            kerning: true,
            line_height: 1.2,
            color: Color::from_rgba8(0, 0, 0, 255),
        }
    }

    /// Parley's properties for this style.
    pub(crate) fn properties(&self) -> Vec<StyleProperty<'a, Color>> {
        vec![
            StyleProperty::FontFamily(FontFamily::named(self.family.name())),
            StyleProperty::FontSize(self.size),
            StyleProperty::FontWeight(parley::FontWeight::new(self.weight)),
            StyleProperty::FontVariations(FontVariations::List(Cow::Owned(self.variations()))),
            StyleProperty::FontFeatures(FontFeatures::List(Cow::Owned(self.features()))),
            StyleProperty::LineHeight(LineHeight::FontSizeRelative(self.line_height)),
            StyleProperty::Brush(self.color),
        ]
    }

    fn variations(&self) -> Vec<FontVariation> {
        let mut variations = vec![FontVariation::new(Tag::new(b"wght"), self.weight)];
        if let Some(opsz) = self.optical_size {
            variations.push(FontVariation::new(Tag::new(b"opsz"), opsz));
        }
        variations
    }

    fn features(&self) -> Vec<FontFeature> {
        vec![
            FontFeature::new(Tag::new(b"tnum"), u16::from(self.tabular_figures)),
            FontFeature::new(Tag::new(b"kern"), u16::from(self.kerning)),
        ]
    }
}

/// A style applied to a byte range of the text.
#[derive(Clone, Debug, PartialEq)]
pub struct Span<'a> {
    /// Byte range in the text.
    pub range: Range<usize>,
    /// The style for that range.
    pub style: Style<'a>,
}

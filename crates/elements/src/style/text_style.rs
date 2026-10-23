//! An element's computed text properties as the text crate's style: the
//! face, size, weight, line height and ink of the text runs it holds.

use vapors_css::value::{Color, LengthUnit, Value};
use vapors_scene::Color as Rgba;
use vapors_text::{Align, Family, LineHeight, Style};

use super::values::{Reader, keyword, later, shape};

/// The faces vapors bundles, the only ones named until system fonts by
/// name arrive (a slice after F2c).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Face {
    Geist,
    GeistMono,
}

/// The text style of an element's own text runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextStyle {
    pub(crate) face: Face,
    pub(crate) size: f32,
    pub(crate) weight: f32,
    pub(crate) line_height: LineHeight,
    pub(crate) color: Rgba,
    /// `letter-spacing`, CSS pixels.
    pub(crate) letter_spacing: f32,
    /// `text-align`: how a block lines up its inline content.
    pub(crate) align: Align,
    /// `font-style: italic` or `oblique`.
    pub(crate) italic: bool,
}

impl TextStyle {
    /// The text crate's style for this.
    pub(crate) fn for_layout(&self) -> Style<'static> {
        let family = match self.face {
            Face::Geist => Family::Geist,
            Face::GeistMono => Family::GeistMono,
        };
        Style {
            weight: self.weight,
            line_height: self.line_height,
            color: self.color,
            letter_spacing: self.letter_spacing,
            italic: self.italic,
            ..Style::new(family, self.size)
        }
    }
}

/// The text style of the element `reader` reads.
#[allow(clippy::cast_possible_truncation)] // font sizes are far inside f32
pub(crate) fn text_style(reader: &mut Reader<'_>) -> TextStyle {
    reader.read("white-space", white_space, ());
    let size = reader.read("font-size", font_size, 16.0);
    TextStyle {
        face: reader.read("font-family", face, Face::Geist),
        size,
        weight: reader.read("font-weight", weight, 400.0),
        line_height: reader.read("line-height", line_height, LineHeight::Normal),
        color: reader.read("color", color, Rgba::default()),
        letter_spacing: reader.read("letter-spacing", letter_spacing, 0.0),
        align: reader.read("text-align", align, Align::Start),
        italic: reader.read("font-style", italic, false),
    }
}

#[allow(clippy::cast_possible_truncation)]
fn font_size(value: &Value) -> Result<f32, String> {
    match value {
        Value::Length(length) if length.unit == LengthUnit::Px => Ok(length.value as f32),
        _ => Err(shape("a computed size in px")),
    }
}

/// The first family vapors has. Generic names map to the bundled faces;
/// a family by name other than Geist's waits for system fonts by name.
fn face(value: &Value) -> Result<Face, String> {
    let families = match value {
        Value::CommaList(items) => items.as_slice(),
        single => std::slice::from_ref(single),
    };
    let first = families.first().ok_or("is an empty family list")?;
    let name = match first {
        Value::Keyword(name) | Value::String(name) => name.to_ascii_lowercase(),
        Value::List(words) => words
            .iter()
            .map(Value::to_css)
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase(),
        _ => return Err(shape("a family name")),
    };
    match name.as_str() {
        "geist" | "sans-serif" | "system-ui" | "ui-sans-serif" => Ok(Face::Geist),
        "geist mono" | "monospace" | "ui-monospace" => Ok(Face::GeistMono),
        _ => Err(format!(
            "names a family vapors does not bundle; use Geist, Geist Mono, sans-serif or monospace until system fonts by name {}",
            later("a slice after F2c")
        )),
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // weights are 1 to 1000
fn weight(value: &Value) -> Result<f32, String> {
    match value {
        Value::Number(n) => Ok(*n as f32),
        Value::Integer(n) => Ok(*n as f32),
        Value::Keyword(k) if k == "normal" => Ok(400.0),
        Value::Keyword(k) if k == "bold" => Ok(700.0),
        Value::Keyword(_) => Err(format!(
            "is relative to the parent's weight, which {}",
            later("a slice after F2c")
        )),
        _ => Err(shape("a weight")),
    }
}

#[allow(clippy::cast_possible_truncation)]
fn line_height(value: &Value) -> Result<LineHeight, String> {
    match value {
        Value::Keyword(k) if k == "normal" => Ok(LineHeight::Normal),
        Value::Number(n) => Ok(LineHeight::FontSizeRelative(*n as f32)),
        Value::Length(length) if length.unit == LengthUnit::Px => {
            Ok(LineHeight::Px(length.value as f32))
        }
        _ => Err(shape("a line height the tree reads")),
    }
}

pub(super) fn color(value: &Value) -> Result<Rgba, String> {
    match value {
        Value::Color(Color::Rgba(rgba)) => Ok(*rgba),
        _ => Err(shape("a resolved colour")),
    }
}

/// `italic` and `oblique` are both drawn italic: the bundled faces have
/// no italic, so either is the synthesised slant Chromium also draws.
/// An oblique angle other than the default is refused.
fn italic(value: &Value) -> Result<bool, String> {
    match value {
        Value::Keyword(k) if k == "normal" => Ok(false),
        Value::Keyword(k) if k == "italic" || k == "oblique" => Ok(true),
        _ => Err(shape("`normal`, `italic` or `oblique` without an angle")),
    }
}

#[allow(clippy::cast_possible_truncation)]
fn letter_spacing(value: &Value) -> Result<f32, String> {
    match value {
        Value::Keyword(k) if k == "normal" => Ok(0.0),
        Value::Length(length) if length.unit == LengthUnit::Px => Ok(length.value as f32),
        _ => Err(shape("`normal` or a length in px")),
    }
}

/// Left-to-right only, as vapors lays text out now: `left` is the
/// start edge and `right` the end.
fn align(value: &Value) -> Result<Align, String> {
    match keyword(value)? {
        "start" | "left" => Ok(Align::Start),
        "end" | "right" => Ok(Align::End),
        "center" => Ok(Align::Center),
        "justify" => Ok(Align::Justify),
        other => Err(format!(
            "`{other}` is not a text-align vapors-elements reads; use start, end, left, right, center or justify"
        )),
    }
}

fn white_space(value: &Value) -> Result<(), String> {
    match keyword(value)? {
        "normal" => Ok(()),
        _ => Err(later("a slice after F2c")),
    }
}

//! An element's computed text properties as the text crate's style: the
//! face, size, weight, line height and ink of the text runs it holds.

use vapors_css::value::{Color, LengthUnit, Value};
use vapors_scene::Color as Rgba;
use vapors_text::{Family, LineHeight, Style};

use super::values::{Reader, keyword, later};

/// The faces vapors bundles; the only ones named before F2c S6.
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
    }
}

#[allow(clippy::cast_possible_truncation)]
fn font_size(value: &Value) -> Result<f32, String> {
    match value {
        Value::Length(length) if length.unit == LengthUnit::Px => Ok(length.value as f32),
        other => Err(format!("is not a computed size in px ({other:?})")),
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
        other => return Err(format!("is not a family name ({other:?})")),
    };
    match name.as_str() {
        "geist" | "sans-serif" | "system-ui" | "ui-sans-serif" => Ok(Face::Geist),
        "geist mono" | "monospace" | "ui-monospace" => Ok(Face::GeistMono),
        _ => Err(format!(
            "names a family vapors does not bundle; use Geist, Geist Mono, sans-serif or monospace until system fonts by name {}",
            later("F2c S6")
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
            later("F2c S6")
        )),
        other => Err(format!("is not a weight ({other:?})")),
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
        other => Err(format!("is not a line height the tree reads ({other:?})")),
    }
}

pub(super) fn color(value: &Value) -> Result<Rgba, String> {
    match value {
        Value::Color(Color::Rgba(rgba)) => Ok(*rgba),
        other => Err(format!("is not a resolved colour ({other:?})")),
    }
}

fn white_space(value: &Value) -> Result<(), String> {
    match keyword(value)? {
        "normal" => Ok(()),
        _ => Err(later("F2c S6")),
    }
}

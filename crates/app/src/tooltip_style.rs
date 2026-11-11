//! The tooltip's look, read from `--tooltip-*` custom properties on the
//! root element, which every element inherits. A property the app does
//! not set takes the default below; a value that is not of the
//! property's kind is refused, naming the property and the value.

use vapors_css::ast::ComponentValue;
use vapors_css::serialize::component_values;
use vapors_css::token::TokenKind;
use vapors_css::value::{ColorScheme, parse_color};
use vapors_elements::Document;
use vapors_scene::Color;

use crate::tooltip_error::TooltipError;

// vapors has no design tokens: these defaults are its own choice. An app
// sets the properties from its own tokens (touché from design/tokens.json).
// The padding plus the 1px border make Chromium's tooltip insets
// (`TooltipViewAura`, `kBorderInset` = 4, 8, 5, 8 with the border in them).
const DEFAULT_BACKGROUND: [u8; 4] = [0x2b, 0x2b, 0x2b, 0xff];
const DEFAULT_COLOR: [u8; 4] = [0xf2, 0xf2, 0xf2, 0xff];
const DEFAULT_BORDER_COLOR: [u8; 4] = [0x5c, 0x5c, 0x5c, 0xff];
const DEFAULT_FONT_SIZE: f32 = 12.0;
const DEFAULT_PADDING: [f32; 4] = [3.0, 7.0, 4.0, 7.0];
const DEFAULT_BORDER_WIDTH: f32 = 1.0;
const DEFAULT_BORDER_RADIUS: f32 = 4.0;

/// The tooltip's look, lengths in CSS pixels.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TooltipStyle {
    pub background: Color,
    pub color: Color,
    pub border_color: Color,
    pub font_size: f32,
    /// Top, right, bottom, left, as CSS `padding`.
    pub padding: [f32; 4],
    pub border_width: f32,
    pub border_radius: f32,
}

impl TooltipStyle {
    /// The root's `--tooltip-*` properties as the last update computed
    /// them, each absent one at its default.
    pub(crate) fn of(document: &Document) -> Result<Self, TooltipError> {
        let root = document
            .computed_style(document.root())
            .ok_or(TooltipError::NotUpdated)?;
        let read = Reader {
            custom: |name: &str| root.custom(name),
            current: root.color(),
        };
        Ok(Self {
            background: read.color("--tooltip-background", DEFAULT_BACKGROUND)?,
            color: read.color("--tooltip-color", DEFAULT_COLOR)?,
            border_color: read.color("--tooltip-border-color", DEFAULT_BORDER_COLOR)?,
            font_size: read.length("--tooltip-font-size", DEFAULT_FONT_SIZE)?,
            padding: read.padding("--tooltip-padding", DEFAULT_PADDING)?,
            border_width: read.length("--tooltip-border-width", DEFAULT_BORDER_WIDTH)?,
            border_radius: read.length("--tooltip-border-radius", DEFAULT_BORDER_RADIUS)?,
        })
    }
}

/// Refuses the root's `--tooltip-*` properties when one holds a value of
/// the wrong kind, as the last update computed them. `ElementsApp` calls
/// it after each update, so a bad value fails the first frame, not the
/// first hover; an app driving its own frames calls it there too.
///
/// # Errors
///
/// [`TooltipError::Property`] naming the property and its value, or
/// [`TooltipError::NotUpdated`] before the first update.
pub fn check_tooltip_style(document: &Document) -> Result<(), TooltipError> {
    TooltipStyle::of(document).map(|_| ())
}

/// Reads one custom property at a time from the root's style.
struct Reader<F> {
    custom: F,
    /// The root's `color`, for `currentcolor`.
    current: Color,
}

impl<'a, F: Fn(&str) -> Option<&'a [ComponentValue]>> Reader<F> {
    fn color(&self, name: &'static str, default: [u8; 4]) -> Result<Color, TooltipError> {
        let Some(values) = (self.custom)(name) else {
            let [r, g, b, a] = default;
            return Ok(Color::from_rgba8(r, g, b, a));
        };
        let parsed = parse_color(&significant(values))
            .map_err(|_| refused(name, values, "a colour, such as #202124"))?;
        // the document computes every colour under the light scheme
        // (vapors-elements' `update::environment`), so this does too
        Ok(parsed.resolve(ColorScheme::Light, self.current))
    }

    fn length(&self, name: &'static str, default: f32) -> Result<f32, TooltipError> {
        let Some(values) = (self.custom)(name) else {
            return Ok(default);
        };
        match lengths(values).as_deref() {
            Some([one]) => Ok(*one),
            _ => Err(refused(name, values, "one length in px, such as 4px")),
        }
    }

    fn padding(&self, name: &'static str, default: [f32; 4]) -> Result<[f32; 4], TooltipError> {
        let Some(values) = (self.custom)(name) else {
            return Ok(default);
        };
        let expected = "one to four lengths in px, as CSS padding: 3px 7px";
        match lengths(values).as_deref() {
            Some(&[all]) => Ok([all; 4]),
            Some(&[block, inline]) => Ok([block, inline, block, inline]),
            Some(&[top, inline, bottom]) => Ok([top, inline, bottom, inline]),
            Some(&[top, right, bottom, left]) => Ok([top, right, bottom, left]),
            _ => Err(refused(name, values, expected)),
        }
    }
}

/// `values` without whitespace.
fn significant(values: &[ComponentValue]) -> Vec<ComponentValue> {
    let blank = |value: &&ComponentValue| matches!(value, ComponentValue::Token(token) if token.kind == TokenKind::Whitespace);
    values.iter().filter(|v| !blank(v)).cloned().collect()
}

/// Each value as px when every one is a length in px of 0 or more (or
/// a bare 0); `None` otherwise.
#[allow(clippy::cast_possible_truncation)] // lengths a window can hold
fn lengths(values: &[ComponentValue]) -> Option<Vec<f32>> {
    let px = |value: &ComponentValue| match value {
        ComponentValue::Token(token) => match &token.kind {
            TokenKind::Dimension { value, unit } if unit == "px" => Some(*value),
            TokenKind::Number { value, .. } if *value == 0.0 => Some(0.0),
            _ => None,
        },
        _ => None,
    };
    let all: Option<Vec<f64>> = significant(values).iter().map(px).collect();
    let all = all.filter(|all| all.iter().all(|v| v.is_finite() && *v >= 0.0))?;
    Some(all.into_iter().map(|v| v as f32).collect())
}

fn refused(name: &'static str, values: &[ComponentValue], expected: &'static str) -> TooltipError {
    TooltipError::Property {
        name,
        value: component_values(values).trim().to_owned(),
        expected,
    }
}

#[cfg(test)]
#[path = "tooltip_style_tests.rs"]
mod tests;

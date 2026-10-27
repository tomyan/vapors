//! A control's parts (3-E3, Q1 (c), first half): a range's thumb and
//! track are drawn from prefixed custom properties, so an app's tokens
//! set them on any ancestor (`:root { --vapors-thumb-color: ... }`)
//! and they inherit. The user-agent values live here, not in `ua.css`:
//! a declaration on the input itself would beat an inherited one. A
//! value of another shape is refused where it is declared, naming the
//! shape wanted. `::part()` is the second half, later.

use vapors_css::ast::ComponentValue;
use vapors_css::value::{Color, Length, LengthUnit, parse_color};
use vapors_scene::Color as Rgba;

use super::values::{Refusal, Styles};

/// The thumb's diameter, CSS px.
pub(crate) const THUMB_SIZE: &str = "--vapors-thumb-size";
/// The thumb's colour.
pub(crate) const THUMB_COLOR: &str = "--vapors-thumb-color";
/// The track's thickness, CSS px.
pub(crate) const TRACK_HEIGHT: &str = "--vapors-track-height";
/// The track's colour.
pub(crate) const TRACK_COLOR: &str = "--vapors-track-color";

/// How a range's thumb and track are drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Parts {
    pub(crate) thumb_size: f32,
    pub(crate) thumb_color: Rgba,
    pub(crate) track_height: f32,
    pub(crate) track_color: Rgba,
}

/// The user-agent parts: a 16 px thumb in the focus ring's blue
/// (#0060df) on a 4 px grey (#d0d0d7) track; the thumb's size is
/// Chromium's, so a press lands on the same value in both.
pub(crate) const DEFAULT: Parts = Parts {
    thumb_size: 16.0,
    thumb_color: Rgba {
        r: 0.0,
        g: 96.0 / 255.0,
        b: 223.0 / 255.0,
        a: 1.0,
    },
    track_height: 4.0,
    track_color: Rgba {
        r: 208.0 / 255.0,
        g: 208.0 / 255.0,
        b: 215.0 / 255.0,
        a: 1.0,
    },
};

/// The parts `styles` give, each custom property read where it is
/// set; a refusal for each of another shape.
pub(crate) fn parts(styles: Styles<'_>, refusals: &mut Vec<Refusal>) -> Parts {
    let mut parts = DEFAULT;
    if let Some(size) = read(styles, THUMB_SIZE, px, refusals) {
        parts.thumb_size = size;
    }
    if let Some(height) = read(styles, TRACK_HEIGHT, px, refusals) {
        parts.track_height = height;
    }
    if let Some(color) = read(styles, THUMB_COLOR, rgba, refusals) {
        parts.thumb_color = color;
    }
    if let Some(color) = read(styles, TRACK_COLOR, rgba, refusals) {
        parts.track_color = color;
    }
    parts
}

/// `property`'s value through `parse`, or `None` where it is not set
/// or is refused.
fn read<T>(
    styles: Styles<'_>,
    property: &'static str,
    parse: fn(&[ComponentValue]) -> Result<T, String>,
    refusals: &mut Vec<Refusal>,
) -> Option<T> {
    let values = styles.own.custom(property)?;
    match parse(values) {
        Ok(value) => Some(value),
        Err(reason) => {
            refusals.extend(refusal(styles, property, values, reason));
            None
        }
    }
}

/// The refusal, at the element that declared the value: not where it
/// is inherited unchanged.
fn refusal(
    styles: Styles<'_>,
    property: &'static str,
    values: &[ComponentValue],
    reason: String,
) -> Option<Refusal> {
    let inherited = styles
        .parent
        .is_some_and(|parent| parent.custom(property) == Some(values));
    (!inherited).then(|| Refusal {
        property,
        value: vapors_css::serialize::component_values(values)
            .trim()
            .to_owned(),
        reason,
        declared: None,
    })
}

/// A length in px, not negative.
#[allow(clippy::cast_possible_truncation)] // CSS lengths are far inside f32
fn px(values: &[ComponentValue]) -> Result<f32, String> {
    let wanted = "is not a length in px: a part's size is a px length, such as `16px`";
    let [value] = &meaningful(values)[..] else {
        return Err(wanted.to_owned());
    };
    match Length::parse(value) {
        Ok(Length {
            value,
            unit: LengthUnit::Px,
        }) if value >= 0.0 => Ok(value as f32),
        _ => Err(wanted.to_owned()),
    }
}

/// An sRGB colour.
fn rgba(values: &[ComponentValue]) -> Result<Rgba, String> {
    match parse_color(&meaningful(values)) {
        Ok(Color::Rgba(rgba)) => Ok(rgba),
        _ => Err(
            "is not a colour: a part's colour is a colour such as `#0060df` or `rgb(0 96 223)` (not `currentColor` or `light-dark()`)"
                .to_owned(),
        ),
    }
}

/// The values without the whitespace around them.
fn meaningful(values: &[ComponentValue]) -> Vec<ComponentValue> {
    values
        .iter()
        .filter(|value| !value.is_whitespace())
        .cloned()
        .collect()
}

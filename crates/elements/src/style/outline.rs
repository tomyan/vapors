//! An element's outline (3-E2b): a solid ring drawn outside the border
//! box at `outline-offset`, not laid out. `auto` (Chromium's native
//! focus ring) and the dashed, dotted and double styles are refused, as
//! vapors draws solid lines only.

use vapors_css::value::Value;
use vapors_scene::Color as Rgba;

use super::decoration::{snap_width, width_px};
use super::text_style::color;
use super::values::{Amount, Reader, amount, keyword, shape};

/// An outline to draw, in CSS pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Outline {
    /// The ring's width, snapped to device pixels as a border's is.
    pub(crate) width: f32,
    /// How far outside the border box the ring starts.
    pub(crate) offset: f32,
    pub(crate) color: Rgba,
}

/// The outline of the element `reader` reads, at `scale` device pixels
/// per CSS pixel; `None` when its style is `none` or its width zero.
pub(crate) fn outline(reader: &mut Reader<'_>, scale: f32) -> Option<Outline> {
    let drawn = reader.read("outline-style", outline_style, false);
    let width = snap_width(reader.read("outline-width", width_px, 0.0), scale);
    let offset = reader.read("outline-offset", offset_px, 0.0);
    let color = reader.read("outline-color", color, Rgba::default());
    (drawn && width > 0.0).then_some(Outline {
        width,
        offset,
        color,
    })
}

fn outline_style(value: &Value) -> Result<bool, String> {
    match keyword(value)? {
        "solid" => Ok(true),
        "none" => Ok(false),
        "auto" => Err("is Chromium's native focus ring, which vapors does not draw; use solid with a width and colour".to_owned()),
        "dashed" | "dotted" | "double" => Err("is not scheduled: vapors draws solid outlines".to_owned()),
        other => Err(super::values::unknown(other)),
    }
}

fn offset_px(value: &Value) -> Result<f32, String> {
    match amount(value)? {
        Amount::Px(px) => Ok(px),
        Amount::Fraction(_) | Amount::Calc(_) => Err(shape("an outline offset in px")),
    }
}

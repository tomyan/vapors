//! Borders, corner radii and box shadows: what an element paints around
//! and under its content. Border widths are snapped to device pixels as
//! Blink snaps them (a width under one device pixel becomes one, any
//! other is floored), so layout and paint use the same widths.

use vapors_css::value::{Color, Value};
use vapors_scene::Color as Rgba;

use super::text_style::color;
use super::values::{Amount, Reader, amount, keyword, later, shape};

const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];
const CORNERS: [&str; 4] = ["top-left", "top-right", "bottom-right", "bottom-left"];

/// One outer box shadow, in CSS pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BoxShadow {
    pub(crate) offset: [f32; 2],
    pub(crate) blur: f32,
    pub(crate) spread: f32,
    pub(crate) color: Rgba,
}

/// An element's border, radii and shadows, in CSS pixels.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Decoration {
    /// Used widths, zero where the style is `none` or `hidden`: top,
    /// right, bottom, left.
    pub(crate) border: [f32; 4],
    pub(crate) border_colors: [Rgba; 4],
    /// Top-left, top-right, bottom-right, bottom-left.
    pub(crate) radii: [f32; 4],
    /// In the order declared: the first is drawn on top.
    pub(crate) shadows: Vec<BoxShadow>,
    /// Its outline, if it draws one (3-E2b).
    pub(crate) outline: Option<super::outline::Outline>,
}

/// The decoration of the element `reader` reads, at `scale` device
/// pixels per CSS pixel; `current` is its `color`.
pub(crate) fn decoration(reader: &mut Reader<'_>, scale: f32, current: Rgba) -> Decoration {
    let mut out = Decoration::default();
    for (i, side) in SIDES.iter().enumerate() {
        out.border[i] = border_width(reader, side, scale);
        out.border_colors[i] = reader.read(side_property(side, "color"), color, Rgba::default());
    }
    for (i, corner) in CORNERS.iter().enumerate() {
        out.radii[i] = reader.read(corner_property(corner), radius, 0.0);
    }
    out.shadows = reader.read("box-shadow", |v| shadows(v, current), Vec::new());
    out.outline = super::outline::outline(reader, scale);
    refuse_elliptical_inner_corners(reader, &out);
    out
}

/// A rounded corner whose two borders differ in width has an elliptical
/// inner edge (CSS Backgrounds 3 §5.2: each axis loses its own width);
/// vapors draws circular corners, so it is refused.
fn refuse_elliptical_inner_corners(reader: &mut Reader<'_>, decoration: &Decoration) {
    // Each corner with its two sides, as indices into `border`.
    let meeting = [(0, 3), (0, 1), (2, 1), (2, 3)];
    for (i, (a, b)) in meeting.into_iter().enumerate() {
        let (wa, wb) = (decoration.border[a], decoration.border[b]);
        if decoration.radii[i] > 0.0 && (wa - wb).abs() > f32::EPSILON {
            reader.refuse(
                corner_property(CORNERS[i]),
                format!(
                    "rounds a corner where a {wa}px and a {wb}px border meet, which makes an elliptical inner corner; vapors draws circular corners, so give the two borders one width, or this corner no radius"
                ),
            );
        }
    }
}

fn border_width(reader: &mut Reader<'_>, side: &str, scale: f32) -> f32 {
    let drawn = reader.read(side_property(side, "style"), border_style, false);
    let width = reader.read(side_property(side, "width"), width_px, 0.0);
    if drawn { snap_width(width, scale) } else { 0.0 }
}

/// Blink's border-width snapping: under one device pixel (and more
/// than zero) is one device pixel; otherwise floored to whole device
/// pixels.
pub(crate) fn snap_width(width: f32, scale: f32) -> f32 {
    let device = width * scale;
    if device <= 0.0 {
        0.0
    } else if device < 1.0 {
        1.0 / scale
    } else {
        device.floor() / scale
    }
}

fn side_property(side: &str, what: &str) -> &'static str {
    let name = format!("border-{side}-{what}");
    vapors_css::property::PropertyId::from_name(&name)
        .unwrap_or_else(|| panic!("`{name}` is not a longhand"))
        .name()
}

fn corner_property(corner: &str) -> &'static str {
    side_property(corner, "radius")
}

fn border_style(value: &Value) -> Result<bool, String> {
    match keyword(value)? {
        "solid" => Ok(true),
        "none" | "hidden" => Ok(false),
        "dashed" | "dotted" | "double" => {
            Err("is not scheduled in docs/F2c.md: vapors draws solid borders".to_owned())
        }
        other => Err(super::values::unknown(other)),
    }
}

#[allow(clippy::cast_possible_truncation)] // CSS lengths are far inside f32
pub(crate) fn width_px(value: &Value) -> Result<f32, String> {
    match value {
        Value::Keyword(k) if k == "thin" => Ok(1.0),
        Value::Keyword(k) if k == "medium" => Ok(3.0),
        Value::Keyword(k) if k == "thick" => Ok(5.0),
        _ => match amount(value)? {
            Amount::Px(px) => Ok(px),
            Amount::Fraction(_) | Amount::Calc(_) => Err(shape("a border width in px")),
        },
    }
}

fn radius(value: &Value) -> Result<f32, String> {
    match amount(value)? {
        Amount::Px(px) => Ok(px),
        Amount::Fraction(_) | Amount::Calc(_) => Err(format!(
            "is a percentage, which makes elliptical corners on a box that is not square; vapors draws circular corners, so give the radius in px (a large one, such as 9999px, makes a pill), until {}",
            later("a slice after F2c")
        )),
    }
}

fn shadows(value: &Value, current: Rgba) -> Result<Vec<BoxShadow>, String> {
    let list = match value {
        Value::Keyword(k) if k == "none" => return Ok(Vec::new()),
        Value::CommaList(items) => items.as_slice(),
        single => std::slice::from_ref(single),
    };
    list.iter()
        .map(|shadow| one_shadow(shadow, current))
        .collect()
}

#[allow(clippy::cast_possible_truncation)]
fn one_shadow(value: &Value, current: Rgba) -> Result<BoxShadow, String> {
    let Value::List(parts) = value else {
        return Err(shape("a shadow"));
    };
    let mut lengths = Vec::new();
    let mut colour = current;
    for part in parts {
        match part {
            Value::Keyword(k) if k == "inset" => {
                return Err(format!(
                    "is an inset shadow, which {}",
                    later("a slice after F2c")
                ));
            }
            Value::Color(Color::Rgba(rgba)) => colour = *rgba,
            other => match amount(other)? {
                Amount::Px(px) => lengths.push(px),
                Amount::Fraction(_) | Amount::Calc(_) => return Err(shape("a shadow length")),
            },
        }
    }
    let at = |i: usize| lengths.get(i).copied().unwrap_or(0.0);
    if at(2) < 0.0 {
        return Err(format!(
            "has a negative blur radius ({} px), which CSS does not allow",
            at(2)
        ));
    }
    Ok(BoxShadow {
        offset: [at(0), at(1)],
        blur: at(2),
        spread: at(3),
        color: colour,
    })
}

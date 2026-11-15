//! `<rect>` and `<circle>` (one-scene S4b): the attributes each takes,
//! how each value is read (`basic_shape_read`), and an outline
//! equivalent to SVG 2's (10.2, 10.3). For a rect it is SVG's path. For
//! a circle it is two half arcs where SVG has four quarter arcs, which
//! fill and stroke alike until dashes or markers are built. The
//! classifier reads the outline back as its closed form. The compiler
//! refuses a literal value from here at build time, naming its line.
//! The element tree refuses the rest at update time: a bound value, or
//! one set by `set_attribute`. Both read through [`refusal`] and
//! [`not_taken`], so the two give one answer. A size of exactly 0
//! draws nothing, as in SVG and Chromium: [`Outline::Nothing`], not a
//! refusal.

use crate::basic_shape_read::{read, read_all};

/// The paint attributes a basic shape takes, as the cascade's
/// `PRESENTATION_ATTRIBUTES` (a test in vapors-elements holds the two
/// equal; this crate does not depend on vapors-css).
pub const PAINT_ATTRIBUTES: [&str; 3] = ["fill", "stroke", "stroke-width"];

const RECT: [&str; 9] = [
    "x",
    "y",
    "width",
    "height",
    "rx",
    "ry",
    "fill",
    "stroke",
    "stroke-width",
];
const CIRCLE: [&str; 6] = ["cx", "cy", "r", "fill", "stroke", "stroke-width"];

/// What a basic shape draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outline {
    /// The outline, as path data.
    Path(String),
    /// Nothing: a width, height or radius of exactly 0.
    Nothing,
}

/// Every attribute a `tag` element takes: its geometry, then its paint
/// ([`PAINT_ATTRIBUTES`]); `None` for a tag that is not a basic shape.
#[must_use]
pub fn allowed(tag: &str) -> Option<&'static [&'static str]> {
    match tag {
        "rect" => Some(&RECT),
        "circle" => Some(&CIRCLE),
        _ => None,
    }
}

/// The geometry attributes of a `tag` element: [`allowed`] less its
/// paint.
#[must_use]
pub fn geometry_names(tag: &str) -> Option<&'static [&'static str]> {
    allowed(tag).map(|all| &all[..all.len() - PAINT_ATTRIBUTES.len()])
}

/// Why the geometry attribute `name` with `value` (`None`: absent) is
/// refused: not a bare number, negative, or a required size absent.
#[must_use]
pub fn refusal(name: &str, value: Option<&str>) -> Option<String> {
    read(name, value).err()
}

/// Why `name` is refused on a `tag` element, which does not take it.
///
/// # Panics
/// When `tag` is not a basic shape: the caller asks [`allowed`] first.
#[must_use]
pub fn not_taken(tag: &str, name: &str) -> String {
    let takes = allowed(tag).unwrap_or_else(|| panic!("<{tag}> is not a basic shape"));
    format!(
        "`{name}` is not an attribute of <{tag}>, which takes {}; remove it",
        takes.join(", ")
    )
}

/// What a `tag` element draws, `attribute` giving each attribute's
/// value.
///
/// # Errors
/// The first value [`refusal`] refuses, or a `tag` that is not a basic
/// shape.
pub fn outline<'a>(
    tag: &str,
    attribute: impl Fn(&str) -> Option<&'a str>,
) -> Result<Outline, String> {
    match tag {
        "rect" => rect(&attribute),
        "circle" => circle(&attribute),
        _ => Err(format!("<{tag}> is not a basic shape")),
    }
}

/// SVG 2's rect outline: square corners unless both radii are above
/// zero (one radius of 0 draws square corners, as SVG does).
fn rect<'a>(attribute: &impl Fn(&str) -> Option<&'a str>) -> Result<Outline, String> {
    let [x, y, w, h] = read_all(attribute, ["x", "y", "width", "height"])?;
    let [rx, ry] = corner_radii(attribute, [w, h])?;
    // read refuses a negative size, so <= 0 is exactly 0
    if w <= 0.0 || h <= 0.0 {
        return Ok(Outline::Nothing);
    }
    if rx <= 0.0 || ry <= 0.0 {
        return Ok(Outline::Path(format!("M{x} {y}H{}V{}H{x}Z", x + w, y + h)));
    }
    Ok(Outline::Path(rounded([x, y, w, h], [rx, ry])))
}

/// From (x + rx, y) clockwise, a quarter arc at each corner.
fn rounded([x, y, w, h]: [f32; 4], [rx, ry]: [f32; 2]) -> String {
    let arc = |to_x: f32, to_y: f32| format!("A{rx} {ry} 0 0 1 {to_x} {to_y}");
    let (right, bottom) = (x + w, y + h);
    let top = format!("M{} {y}H{}{}", x + rx, right - rx, arc(right, y + ry));
    let right_side = format!("V{}{}", bottom - ry, arc(right - rx, bottom));
    let bottom_side = format!("H{}{}", x + rx, arc(x, bottom - ry));
    format!(
        "{top}{right_side}{bottom_side}V{}{}Z",
        y + ry,
        arc(x + rx, y)
    )
}

/// A circle as two half arcs from its leftmost point.
fn circle<'a>(attribute: &impl Fn(&str) -> Option<&'a str>) -> Result<Outline, String> {
    let [cx, cy, r] = read_all(attribute, ["cx", "cy", "r"])?;
    if r <= 0.0 {
        return Ok(Outline::Nothing);
    }
    let (left, right) = (cx - r, cx + r);
    Ok(Outline::Path(format!(
        "M{left} {cy}A{r} {r} 0 1 0 {right} {cy}A{r} {r} 0 1 0 {left} {cy}Z"
    )))
}

/// The radii SVG 2 uses (10.2, auto): one given stands for both, and
/// each is clamped to half its side.
fn corner_radii<'a>(
    attribute: &impl Fn(&str) -> Option<&'a str>,
    [w, h]: [f32; 2],
) -> Result<[f32; 2], String> {
    let given = |name| attribute(name).map(|text| read(name, Some(text)));
    let (rx, ry) = match (given("rx").transpose()?, given("ry").transpose()?) {
        (Some(rx), Some(ry)) => (rx, ry),
        (Some(r), None) | (None, Some(r)) => (r, r),
        (None, None) => (0.0, 0.0),
    };
    Ok([rx.min(w / 2.0), ry.min(h / 2.0)])
}

#[cfg(test)]
#[path = "basic_shape_tests.rs"]
mod tests;

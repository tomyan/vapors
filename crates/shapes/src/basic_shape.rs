//! `<rect>` and `<circle>` (one-scene S4b): the attributes each takes,
//! how each attribute's value is read, and the path SVG 2 gives each
//! (10.2, 10.3). The classifier reads that path back as its closed
//! form: a rect with square or round corners, or a circle. The
//! compiler refuses a literal value from here at build time, naming its
//! line. The element tree refuses the rest at update time: a bound
//! value, or one set by `set_attribute`. Both read through
//! [`refusal`], so the two give one answer.

/// The geometry attributes of a `tag` element; `None` for a tag that
/// is not a basic shape.
#[must_use]
pub fn geometry_names(tag: &str) -> Option<&'static [&'static str]> {
    match tag {
        "rect" => Some(&["x", "y", "width", "height", "rx", "ry"]),
        "circle" => Some(&["cx", "cy", "r"]),
        _ => None,
    }
}

/// Every attribute a `tag` element takes: its geometry and its paint.
#[must_use]
pub fn allowed(tag: &str) -> Option<&'static [&'static str]> {
    match tag {
        "rect" => Some(&[
            "x",
            "y",
            "width",
            "height",
            "rx",
            "ry",
            "fill",
            "stroke",
            "stroke-width",
        ]),
        "circle" => Some(&["cx", "cy", "r", "fill", "stroke", "stroke-width"]),
        _ => None,
    }
}

/// Why the geometry attribute `name` with `value` (`None`: absent) is
/// refused: not a bare number, out of range, or a required one absent.
#[must_use]
pub fn refusal(name: &str, value: Option<&str>) -> Option<String> {
    read(name, value).err()
}

/// The path data of the outline a `tag` element draws, `attribute`
/// giving each attribute's value; `None` when one is refused.
pub fn outline<'a>(tag: &str, attribute: impl Fn(&str) -> Option<&'a str>) -> Option<String> {
    match tag {
        "rect" => rect(&attribute).ok(),
        "circle" => circle(&attribute).ok(),
        _ => None,
    }
}

/// SVG 2's rect outline: square corners unless both radii are above
/// zero (one radius of 0 draws square corners, as SVG does), then from
/// (x + rx, y) clockwise, a quarter arc at each corner.
fn rect<'a>(attribute: &impl Fn(&str) -> Option<&'a str>) -> Result<String, String> {
    let [x, y, w, h] = read_all(attribute, ["x", "y", "width", "height"])?;
    let [rx, ry] = corner_radii(attribute, [w, h])?;
    if rx <= 0.0 || ry <= 0.0 {
        return Ok(format!("M{x} {y}H{}V{}H{x}Z", x + w, y + h));
    }
    let arc = |to_x: f32, to_y: f32| format!("A{rx} {ry} 0 0 1 {to_x} {to_y}");
    let (right, bottom) = (x + w, y + h);
    Ok(format!(
        "M{} {y}H{}{}V{}{}H{}{}V{}{}Z",
        x + rx,
        right - rx,
        arc(right, y + ry),
        bottom - ry,
        arc(right - rx, bottom),
        x + rx,
        arc(x, bottom - ry),
        y + ry,
        arc(x + rx, y),
    ))
}

/// A circle as two half arcs from its leftmost point.
fn circle<'a>(attribute: &impl Fn(&str) -> Option<&'a str>) -> Result<String, String> {
    let [cx, cy, r] = read_all(attribute, ["cx", "cy", "r"])?;
    Ok(format!(
        "M{} {cy}A{r} {r} 0 1 0 {} {cy}A{r} {r} 0 1 0 {} {cy}Z",
        cx - r,
        cx + r,
        cx - r
    ))
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

/// `name`'s value: a coordinate, 0 when absent; a size, required and
/// above zero; a corner radius, not negative.
fn read(name: &str, text: Option<&str>) -> Result<f32, String> {
    let value = text.map(crate::css_number::css_number);
    match (Kind::of(name), text, value) {
        (Kind::Coordinate | Kind::Radius, None, _) => Ok(0.0),
        (Kind::Size, None, _) => Err(format!(
            "its {name} attribute is required: SVG draws nothing without it; give it in viewBox units, such as {name}=\"4\""
        )),
        (kind, Some(_), Some(Some(value))) if kind.takes(value) => Ok(value),
        (kind, Some(text), _) => Err(format!(
            "its {name} {text:?} is not {} with no unit; give viewBox units as a bare number, such as {name}=\"4\"",
            kind.wanted()
        )),
    }
}

/// What an attribute holds, for its range and its message.
#[derive(Clone, Copy)]
enum Kind {
    Coordinate,
    Size,
    Radius,
}

impl Kind {
    fn of(name: &str) -> Self {
        match name {
            "width" | "height" | "r" => Self::Size,
            "rx" | "ry" => Self::Radius,
            _ => Self::Coordinate,
        }
    }

    fn takes(self, value: f32) -> bool {
        match self {
            Self::Coordinate => true,
            Self::Size => value > 0.0,
            Self::Radius => value >= 0.0,
        }
    }

    fn wanted(self) -> &'static str {
        match self {
            Self::Coordinate => "a number",
            Self::Size => "a number above 0",
            Self::Radius => "a number of 0 or more",
        }
    }
}

/// Each of `names` read, or the first refusal.
fn read_all<'a, const N: usize>(
    attribute: &impl Fn(&str) -> Option<&'a str>,
    names: [&str; N],
) -> Result<[f32; N], String> {
    let mut values = [0.0; N];
    for (value, name) in values.iter_mut().zip(names) {
        *value = read(name, attribute(name))?;
    }
    Ok(values)
}

#[cfg(test)]
#[path = "basic_shape_tests.rs"]
mod tests;

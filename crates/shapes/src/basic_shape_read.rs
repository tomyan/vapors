//! How a basic shape's geometry attribute is read: a bare number in
//! viewBox units (`css_number`), in its range. A coordinate takes any
//! number and is 0 when absent. A size (width, height, r) is required
//! and is 0 or more, where 0 draws nothing, as in SVG. A corner radius
//! is 0 or more and is 0 when absent.

use crate::css_number::css_number;

/// `name`'s value from `text` (`None`: absent), or why it is refused.
pub(crate) fn read(name: &str, text: Option<&str>) -> Result<f32, String> {
    let kind = Kind::of(name);
    let Some(text) = text else {
        return kind.absent(name);
    };
    let value = css_number(text).ok_or_else(|| format!(
        "its {name} {text:?} is not a bare number; give viewBox units with no unit or percentage, such as {name}=\"4\""
    ))?;
    if value < 0.0 && kind != Kind::Coordinate {
        return Err(format!(
            "its {name} {text:?} is negative; give 0 ({}) or more",
            kind.zero()
        ));
    }
    Ok(value)
}

/// Each of `names` read, or the first refusal.
pub(crate) fn read_all<'a, const N: usize>(
    attribute: &impl Fn(&str) -> Option<&'a str>,
    names: [&str; N],
) -> Result<[f32; N], String> {
    let mut values = [0.0; N];
    for (value, name) in values.iter_mut().zip(names) {
        *value = read(name, attribute(name))?;
    }
    Ok(values)
}

/// What an attribute holds, for its range and its messages.
#[derive(Clone, Copy, PartialEq, Eq)]
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

    fn absent(self, name: &str) -> Result<f32, String> {
        match self {
            Self::Size => Err(format!(
                "its {name} attribute is required: SVG draws nothing without it; give it in viewBox units, such as {name}=\"4\""
            )),
            Self::Coordinate | Self::Radius => Ok(0.0),
        }
    }

    fn zero(self) -> &'static str {
        match self {
            Self::Size => "draws nothing",
            Self::Radius | Self::Coordinate => "square corners",
        }
    }
}

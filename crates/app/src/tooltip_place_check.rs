//! The input check for [`place_tooltip`](super::place_tooltip).

use std::fmt;

use vapors_elements::BoxRect;

use super::Anchor;
#[cfg(test)]
use super::Point;

/// Which input of `place_tooltip` held the bad value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementField {
    /// `Anchor::Pointer` x.
    HotspotX,
    /// `Anchor::Pointer` y.
    HotspotY,
    /// `Anchor::Keyboard` x.
    AnchorX,
    /// `Anchor::Keyboard` y.
    AnchorY,
    /// `Anchor::Keyboard` width.
    AnchorWidth,
    /// `Anchor::Keyboard` height.
    AnchorHeight,
    /// The tooltip's width.
    SizeWidth,
    /// The tooltip's height.
    SizeHeight,
    /// The window's width.
    WindowWidth,
    /// The window's height.
    WindowHeight,
}

impl PlacementField {
    /// The name as the caller wrote it, e.g. `size.height`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::HotspotX => "hotspot.x",
            Self::HotspotY => "hotspot.y",
            Self::AnchorX => "anchor.x",
            Self::AnchorY => "anchor.y",
            Self::AnchorWidth => "anchor.width",
            Self::AnchorHeight => "anchor.height",
            Self::SizeWidth => "size.width",
            Self::SizeHeight => "size.height",
            Self::WindowWidth => "window.width",
            Self::WindowHeight => "window.height",
        }
    }

    fn is_length(self) -> bool {
        !matches!(
            self,
            Self::HotspotX | Self::HotspotY | Self::AnchorX | Self::AnchorY
        )
    }
}

/// An input to `place_tooltip` that is not usable: the field and its value.
#[derive(Clone, Copy, Debug)]
pub struct PlacementError {
    /// The field that was refused.
    pub field: PlacementField,
    /// Its value.
    pub value: f32,
}

impl fmt::Display for PlacementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let need = if self.field.is_length() {
            "a finite length of 0 or more"
        } else {
            "a finite coordinate"
        };
        write!(
            f,
            "place_tooltip: `{}` is {}; pass {need} in CSS pixels",
            self.field.name(),
            self.value
        )
    }
}

impl std::error::Error for PlacementError {}

fn field_values(anchor: Anchor) -> Vec<(PlacementField, f32)> {
    match anchor {
        Anchor::Pointer(p) => vec![
            (PlacementField::HotspotX, p.x),
            (PlacementField::HotspotY, p.y),
        ],
        Anchor::Keyboard(BoxRect {
            x,
            y,
            width,
            height,
        }) => vec![
            (PlacementField::AnchorX, x),
            (PlacementField::AnchorY, y),
            (PlacementField::AnchorWidth, width),
            (PlacementField::AnchorHeight, height),
        ],
    }
}

/// The first field that is NaN, infinite, or a negative length.
pub(super) fn check(
    anchor: Anchor,
    size: [f32; 2],
    window: [f32; 2],
) -> Result<(), PlacementError> {
    let rest = [
        (PlacementField::SizeWidth, size[0]),
        (PlacementField::SizeHeight, size[1]),
        (PlacementField::WindowWidth, window[0]),
        (PlacementField::WindowHeight, window[1]),
    ];
    let bad = field_values(anchor)
        .into_iter()
        .chain(rest)
        .find(|(field, v)| !v.is_finite() || (field.is_length() && *v < 0.0));
    match bad {
        Some((field, value)) => Err(PlacementError { field, value }),
        None => Ok(()),
    }
}

#[cfg(test)]
#[path = "tooltip_place_check_tests.rs"]
mod tests;

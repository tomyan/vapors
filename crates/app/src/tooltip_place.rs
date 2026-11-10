//! Where a tooltip's top-left corner goes: [`place_tooltip`]. Pointer
//! placement follows Chromium's `TooltipAura::GetTooltipBounds`
//! (`ui/views/corewm/tooltip_aura.cc`); keyboard placement is vapors'
//! own (see docs/tooltips.md, "Where the tooltip goes").

use std::fmt;

use vapors_elements::BoxRect;

/// Offset of a pointer tooltip's top-left from the hotspot, CSS px:
/// `TooltipAura::kCursorOffsetX`.
pub const CURSOR_OFFSET_X: f32 = 10.0;
/// `TooltipAura::kCursorOffsetY`.
pub const CURSOR_OFFSET_Y: f32 = 15.0;
/// Space between a focused element and its keyboard tooltip, CSS px.
/// Chromium puts the keyboard tooltip's top edge on the anchor point
/// with no gap; vapors' anchor is the border box, so this is a choice.
pub const KEYBOARD_GAP: f32 = 4.0;

/// A point in window CSS pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// Distance from the left edge.
    pub x: f32,
    /// Distance from the top edge.
    pub y: f32,
}

/// What showed the tooltip, and where it points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Trigger {
    /// The pointer rested; its hotspot.
    Pointer(Point),
    /// Keyboard focus; the focused element's border box.
    Keyboard(BoxRect),
}

/// An input to [`place_tooltip`] that is not a usable length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacementError {
    /// Which input: `trigger`, `size` or `window`.
    pub input: &'static str,
}

impl fmt::Display for PlacementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "place_tooltip: `{}` has a value that is NaN, infinite or negative \
             (sizes) - pass finite CSS pixels",
            self.input
        )
    }
}

impl std::error::Error for PlacementError {}

#[derive(Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    size: [f32; 2],
}

impl Rect {
    fn fits_vertically(self, window: [f32; 2]) -> bool {
        self.y >= 0.0 && self.y + self.size[1] <= window[1]
    }

    fn fits_horizontally(self, window: [f32; 2]) -> bool {
        self.x >= 0.0 && self.x + self.size[0] <= window[0]
    }

    /// Moved inside the window; a side longer than the window starts
    /// at 0.
    fn clamped(self, window: [f32; 2]) -> Rect {
        let slide = |v: f32, len: f32, limit: f32| v.min(limit - len).max(0.0);
        Rect {
            x: slide(self.x, self.size[0], window[0]),
            y: slide(self.y, self.size[1], window[1]),
            size: self.size,
        }
    }

    fn origin(self) -> Point {
        Point {
            x: self.x,
            y: self.y,
        }
    }
}

/// The tooltip's top-left corner, in window CSS pixels, for a tooltip
/// of `size` in a window of `window`.
///
/// A pointer tooltip sits at the hotspot plus (10, 15), slid left to fit
/// the window, with its bottom edge on the hotspot's row when it does
/// not fit below. When it fits neither below nor above it goes beside
/// the hotspot. A keyboard tooltip is centred under the element's border
/// box with a gap, or above it when it does not fit below. The result is
/// inside the window and does not cover the hotspot, except that a
/// tooltip longer than the window on a side starts at 0 on that side,
/// and one that fits no candidate is placed below and clamped.
///
/// # Errors
/// A NaN or infinite value, or a negative `size` or `window`.
pub fn place_tooltip(
    trigger: Trigger,
    size: [f32; 2],
    window: [f32; 2],
) -> Result<Point, PlacementError> {
    check(trigger, size, window)?;
    Ok(match trigger {
        Trigger::Pointer(hotspot) => pointer(hotspot, size, window),
        Trigger::Keyboard(anchor) => keyboard(anchor, size, window),
    })
}

fn check(trigger: Trigger, size: [f32; 2], window: [f32; 2]) -> Result<(), PlacementError> {
    let lengths = |values: &[f32]| values.iter().all(|v| v.is_finite() && *v >= 0.0);
    let finite = |values: &[f32]| values.iter().all(|v| v.is_finite());
    let trigger_ok = match trigger {
        Trigger::Pointer(p) => finite(&[p.x, p.y]),
        Trigger::Keyboard(r) => finite(&[r.x, r.y]) && lengths(&[r.width, r.height]),
    };
    let refused = |input| Err(PlacementError { input });
    match (trigger_ok, lengths(&size), lengths(&window)) {
        (false, _, _) => refused("trigger"),
        (_, false, _) => refused("size"),
        (_, _, false) => refused("window"),
        _ => Ok(()),
    }
}

fn pointer(hotspot: Point, size: [f32; 2], window: [f32; 2]) -> Point {
    let x = hotspot.x + CURSOR_OFFSET_X;
    let below = Rect {
        x,
        y: hotspot.y + CURSOR_OFFSET_Y,
        size,
    };
    let above = Rect {
        y: hotspot.y - size[1],
        ..below
    };
    let beside_y = below.clamped(window).y;
    let right = Rect {
        y: beside_y,
        ..below
    };
    let left = Rect {
        x: hotspot.x - size[0],
        y: beside_y,
        size,
    };
    let vertical = [below, above]
        .into_iter()
        .find(|r| r.fits_vertically(window));
    let horizontal = [right, left]
        .into_iter()
        .find(|r| r.fits_horizontally(window));
    vertical
        .or(horizontal)
        .unwrap_or(below)
        .clamped(window)
        .origin()
}

fn keyboard(anchor: BoxRect, size: [f32; 2], window: [f32; 2]) -> Point {
    let x = anchor.x + anchor.width / 2.0 - size[0] / 2.0;
    let below = Rect {
        x,
        y: anchor.y + anchor.height + KEYBOARD_GAP,
        size,
    };
    let above = Rect {
        y: anchor.y - KEYBOARD_GAP - size[1],
        ..below
    };
    let chosen = [below, above]
        .into_iter()
        .find(|r| r.fits_vertically(window));
    chosen.unwrap_or(below).clamped(window).origin()
}

#[cfg(test)]
#[path = "tooltip_place_tests.rs"]
mod tests;

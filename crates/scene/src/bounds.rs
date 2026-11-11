//! Where an item can paint, for culling: every pixel it can change on
//! the target, in device pixels. An item whose bounds miss the part of
//! the target its clip lets through changes no pixel there.

use crate::display::{Item, Rect, Shadow};

/// How far past its geometry an item may touch pixels: a clip becomes a
/// scissor rounded to whole pixels (half a pixel either way), and an
/// antialiased edge covers the pixel it crosses.
const FRINGE: f32 = 1.0;

/// Standard deviations of a shadow's Gaussian that its quad covers:
/// past three, the Gaussian is under 0.3% of its peak.
const SHADOW_REACH: f32 = 3.0;

impl Shadow {
    /// The quad the shadow is drawn over: its shape grown by three
    /// standard deviations of its blur (half the CSS blur radius each).
    #[must_use]
    pub fn drawn(&self) -> Rect {
        self.rect.grown(self.blur / 2.0 * SHADOW_REACH)
    }
}

impl Item {
    /// Every pixel the item can paint, with a [`FRINGE`]: a shadow's
    /// blurred quad, a text run's ink, any other item's rect.
    #[must_use]
    pub fn bounds(&self) -> Rect {
        let painted = match *self {
            Item::Shadow(shadow) => shadow.drawn(),
            Item::Quad(quad) => quad.rect,
            Item::Image { rect, .. } | Item::Icon { rect, .. } | Item::Surface { rect, .. } => rect,
            Item::Text { ink, .. } => ink,
        };
        painted.grown(FRINGE)
    }
}

impl Rect {
    /// This rectangle with each edge moved out by `by`.
    #[must_use]
    pub fn grown(self, by: f32) -> Rect {
        Rect::new(
            self.left - by,
            self.top - by,
            self.right + by,
            self.bottom + by,
        )
    }

    /// This rectangle moved by `[dx, dy]`.
    #[must_use]
    pub fn offset(self, [dx, dy]: [f32; 2]) -> Rect {
        Rect::new(
            self.left + dx,
            self.top + dy,
            self.right + dx,
            self.bottom + dy,
        )
    }

    /// Where both rectangles overlap: edges crossed when they do not.
    #[must_use]
    pub fn intersection(self, other: Rect) -> Rect {
        Rect::new(
            self.left.max(other.left),
            self.top.max(other.top),
            self.right.min(other.right),
            self.bottom.min(other.bottom),
        )
    }

    /// True when no area lies inside: an edge pair met or crossed. A
    /// NaN edge compares false, so a rectangle that cannot be placed is
    /// not empty and nothing is culled by it.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }

    /// True when the two share no area. A NaN edge compares false, so
    /// such a rectangle is never disjoint.
    #[must_use]
    pub fn is_disjoint(&self, other: &Rect) -> bool {
        self.right <= other.left
            || other.right <= self.left
            || self.bottom <= other.top
            || other.bottom <= self.top
    }
}

#[cfg(test)]
#[path = "bounds_tests.rs"]
mod tests;

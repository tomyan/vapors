//! An item's clip (flat-scene F3.2, `docs/plans/flat-scene-f3.md` item
//! 13): a rectangle in device pixels with a circular radius at each
//! corner, top-left, top-right, bottom-right, bottom-left, already fitted
//! to the rectangle as CSS scales radii (`clip.wgsl` takes them as
//! given). A clip whose radii are all 0 is a rectangle, which a scissor
//! can apply; a rounded one only the clip table can. A rounded clip cut
//! by a rectangle that does not hold it (a frame's clip on the page)
//! keeps its corners and carries that rectangle as `outer`: the two
//! together are the cut exactly, where one rounded rectangle could not
//! be.

use crate::display::Rect;

/// A clip: `rect` with corner `radii`, device pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clip {
    /// The rectangle.
    pub rect: Rect,
    /// Each corner's radius: top-left, top-right, bottom-right,
    /// bottom-left.
    pub radii: [f32; 4],
    /// A rectangle the clip is also cut to; only a rounded clip has one.
    pub outer: Option<Rect>,
}

impl Clip {
    /// `rect` with square corners.
    #[must_use]
    pub const fn square(rect: Rect) -> Self {
        Self {
            rect,
            radii: [0.0; 4],
            outer: None,
        }
    }

    /// `rect` with corner `radii` (top-left, top-right, bottom-right,
    /// bottom-left), already fitted to it.
    #[must_use]
    pub const fn rounded(rect: Rect, radii: [f32; 4]) -> Self {
        Self {
            rect,
            radii,
            outer: None,
        }
    }

    /// Whether any corner is rounded.
    #[must_use]
    pub fn is_rounded(&self) -> bool {
        self.radii.iter().any(|r| *r > 0.0)
    }

    /// The clip moved by `shift`, device pixels.
    #[must_use]
    pub fn offset(self, shift: [f32; 2]) -> Self {
        Self {
            rect: self.rect.offset(shift),
            outer: self.outer.map(|outer| outer.offset(shift)),
            ..self
        }
    }

    /// The clip cut to `by`: a square clip becomes the intersection; a
    /// rounded one inside `by` is unchanged, and one that is not keeps
    /// its corners and is cut to `by` as `outer`.
    #[must_use]
    pub fn cut(self, by: Rect) -> Self {
        if !self.is_rounded() {
            return Self::square(self.rect.intersection(by));
        }
        if by.contains(&self.rect) {
            return self;
        }
        let outer = self.outer.map_or(by, |outer| outer.intersection(by));
        Self {
            outer: Some(outer),
            ..self
        }
    }

    /// The rectangle nothing the clip lets through falls outside.
    #[must_use]
    pub fn bounds(&self) -> Rect {
        self.outer
            .map_or(self.rect, |outer| self.rect.intersection(outer))
    }
}

impl From<Rect> for Clip {
    fn from(rect: Rect) -> Self {
        Self::square(rect)
    }
}

#[cfg(test)]
#[path = "clip_tests.rs"]
mod tests;

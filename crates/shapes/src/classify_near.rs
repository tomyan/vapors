//! The classifier's tolerance: what "the same position" means.

use zeno::Point;

/// How far a drawn position may differ from the closed form's and
/// still be read as equal, relative to the larger of the shape's size
/// and its coordinates' magnitude (an f32 ulp near 1000 is 6e-5).
/// Below 5 units it is tighter than four-decimal rounding, so a
/// circle written to four decimals there is a path: conservative,
/// never wrong.
pub(crate) const TOLERANCE: f32 = 1e-5;

/// Whether `a` and `b` are the same point within [`TOLERANCE`] of `scale`.
pub(crate) fn near(a: Point, b: Point, scale: f32) -> bool {
    near_value(a.x, b.x, scale) && near_value(a.y, b.y, scale)
}

pub(crate) fn near_value(a: f32, b: f32, scale: f32) -> bool {
    (a - b).abs() <= TOLERANCE * scale.max(1.0)
}

/// The scale a tolerance is relative to: the largest coordinate
/// magnitude among `points`, which bounds the shape's size too.
pub(crate) fn scale_of(points: &[Point]) -> f32 {
    points
        .iter()
        .map(|p| p.x.abs().max(p.y.abs()))
        .fold(1.0, f32::max)
}

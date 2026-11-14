//! The classifier (docs/plans/one-scene.md 1.2): a path that is
//! exactly an axis-aligned rectangle, or two arcs forming one circle or
//! ellipse, is drawn as a closed form; every other path is a mesh.
//! Only exact forms are taken: a shape the closed form would draw
//! differently from the path is a path, which the tessellator draws
//! exactly, so nothing here is refused.

use zeno::Point;

use crate::canonical::Paint;
use crate::classify_near::{TOLERANCE, near, near_value, scale_of};
use crate::path::IconPath;
use crate::segment::Segment;

/// What a path is drawn as.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Classified {
    /// An axis-aligned rectangle with square corners.
    Rect {
        /// Its top-left corner.
        origin: [f32; 2],
        /// Width and height, both above zero.
        size: [f32; 2],
    },
    /// An ellipse; a circle when the radii are equal.
    Ellipse {
        /// Its centre.
        centre: [f32; 2],
        /// Its radii, both above zero.
        radii: [f32; 2],
    },
    /// Anything else: tessellated.
    Path,
}

/// Classifies `path` under `paint`.
#[must_use]
pub fn classify(path: &IconPath, paint: Paint) -> Classified {
    let segments = path.segments();
    if let Some(corners) = rect_corners(segments, paint) {
        return rect(corners);
    }
    if let Some((from, radii, to)) = circle_arcs(segments) {
        return ellipse(from, radii, to);
    }
    Classified::Path
}

/// The four corners when `segments` are exactly a closed rectangle: a
/// move and three lines, then for a fill an optional line back to the
/// start and an optional close, for a stroke a close alone (a line
/// back to the start would add a join Chromium draws).
fn rect_corners(segments: &[Segment], paint: Paint) -> Option<[Point; 4]> {
    let [
        Segment::Move(p0),
        Segment::Line(p1),
        Segment::Line(p2),
        Segment::Line(p3),
        rest @ ..,
    ] = segments
    else {
        return None;
    };
    let closed = match (paint, rest) {
        (_, [Segment::Close]) | (Paint::Fill(_), []) => true,
        (Paint::Fill(_), [Segment::Line(back)] | [Segment::Line(back), Segment::Close]) => {
            near(*back, *p0, scale_of(&[*p0, *p2]))
        }
        _ => false,
    };
    closed.then_some([*p0, *p1, *p2, *p3])
}

/// The rectangle `corners` draw when it has width and height and each
/// side is axis-aligned, the sides alternating; a path otherwise (a
/// rectangle without width or height is a line SVG strokes, which the
/// tessellator draws exactly).
fn rect(corners: [Point; 4]) -> Classified {
    let [p0, _, p2, _] = corners;
    let size = [(p2.x - p0.x).abs(), (p2.y - p0.y).abs()];
    if size[0] <= 0.0 || size[1] <= 0.0 || !sides_alternate(corners) {
        return Classified::Path;
    }
    let origin = [p0.x.min(p2.x), p0.y.min(p2.y)];
    Classified::Rect { origin, size }
}

/// Whether the four sides are axis-aligned and alternate.
fn sides_alternate(corners: [Point; 4]) -> bool {
    let [p0, p1, p2, p3] = corners;
    let scale = scale_of(&corners);
    let horizontal_first = near_value(p0.y, p1.y, scale);
    [(p0, p1), (p1, p2), (p2, p3), (p3, p0)]
        .into_iter()
        .enumerate()
        .all(|(index, (a, b))| {
            if horizontal_first == (index % 2 == 0) {
                near_value(a.y, b.y, scale)
            } else {
                near_value(a.x, b.x, scale)
            }
        })
}

/// The start, the radii and the first arc's end when `segments` are a
/// move and two unrotated arcs of one radius pair, swept the same way,
/// back to the start, closed or not.
fn circle_arcs(segments: &[Segment]) -> Option<(Point, [f32; 2], Point)> {
    let [
        Segment::Move(p0),
        first @ Segment::Arc { to: p1, .. },
        second,
        rest @ ..,
    ] = segments
    else {
        return None;
    };
    if !matches!(rest, [] | [Segment::Close]) {
        return None;
    }
    let scale = scale_of(&[*p0, *p1]);
    let (radii, back) = same_arcs(first, second, scale)?;
    near(back, *p0, scale).then_some((*p0, radii, *p1))
}

/// The radii and the second arc's end when both arcs are unrotated,
/// of one radius pair within the tolerance (the pen scales an arc
/// whose chord is an ulp over the diameter, so relative arcs far from
/// the origin differ by that) and one sweep. Rotations are compared
/// exactly: the source wrote them twice, and a difference is a path.
#[allow(clippy::float_cmp)]
fn same_arcs(first: &Segment, second: &Segment, scale: f32) -> Option<([f32; 2], Point)> {
    let (
        Segment::Arc {
            radii: r1,
            rotation: rot1,
            sweep: s1,
            ..
        },
        Segment::Arc {
            radii: r2,
            rotation: rot2,
            sweep: s2,
            to: back,
            ..
        },
    ) = (first, second)
    else {
        return None;
    };
    let same_radii = near_value(r1[0], r2[0], scale) && near_value(r1[1], r2[1], scale);
    (same_radii && *rot1 == 0.0 && *rot2 == 0.0 && s1 == s2).then_some((*r1, *back))
}

/// The ellipse two half arcs of `radii` draw from `from` to `to` and
/// back, when the chord is a horizontal or vertical diameter: its
/// centre then is the midpoint. A chord short of the diameter puts the
/// two centres `r * sqrt(1 - (chord / 2r)^2)` either side of the
/// midpoint, so that drawn offset, not the chord, is what is held to
/// the tolerance. The pen made the radii positive and at least half
/// the chord.
///
/// # Panics
/// When a radius is not above zero: the pen's guarantee broken.
fn ellipse(from: Point, [rx, ry]: [f32; 2], to: Point) -> Classified {
    assert!(
        rx > 0.0 && ry > 0.0,
        "an arc with radii [{rx}, {ry}] reached the classifier; Pen::push makes a zero-radius arc a line"
    );
    let scale = scale_of(&[from, to]).max(rx).max(ry);
    let offset = if near_value(from.y, to.y, scale) {
        centre_offset((from.x - to.x).abs() / 2.0, rx, ry)
    } else if near_value(from.x, to.x, scale) {
        centre_offset((from.y - to.y).abs() / 2.0, ry, rx)
    } else {
        return Classified::Path;
    };
    if offset > TOLERANCE * scale {
        return Classified::Path;
    }
    let centre = [f32::midpoint(from.x, to.x), f32::midpoint(from.y, to.y)];
    Classified::Ellipse {
        centre,
        radii: [rx, ry],
    }
}

/// How far each arc's centre is from the chord's midpoint, for a half
/// chord `half` along the axis of radius `along`, the other radius
/// `across` (SVG F.6.5 with rotation 0).
fn centre_offset(half: f32, along: f32, across: f32) -> f32 {
    across * (1.0 - (half / along).powi(2)).max(0.0).sqrt()
}

#[cfg(test)]
#[path = "classify_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "classify_oracle_tests.rs"]
mod oracle_tests;

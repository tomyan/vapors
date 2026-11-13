//! The classifier (docs/plans/one-scene.md 1.2): a path that is
//! exactly an axis-aligned rectangle, or two arcs forming one circle or
//! ellipse, is drawn as a closed form; every other path is a mesh.
//! Only exact forms are taken: a shape the closed form would draw
//! differently from the path is a path.

use zeno::Point;

use crate::canonical::Paint;
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

/// How far two coordinates may differ and still be read as equal,
/// relative to the shape's size: the two arcs of a circle written to
/// four decimals meet within it.
const TOLERANCE: f32 = 1e-5;

/// Classifies `path` under `paint`; `site` names the shape in a refusal.
///
/// # Errors
/// A rectangle without width or height: SVG fills nothing and strokes
/// a line, and vapors draws neither as a closed form; the message
/// names the site. A circle cannot lack a radius: the pen turned a
/// zero-radius arc into a line before the segments reached here.
pub fn classify(site: &str, path: &IconPath, paint: Paint) -> Result<Classified, String> {
    let segments = path.segments();
    if let Some(corners) = rect_corners(segments, paint) {
        return rect(site, corners);
    }
    if let Some((from, radii, to)) = circle_arcs(segments) {
        return Ok(ellipse(site, from, radii, to));
    }
    Ok(Classified::Path)
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
            near(*back, *p0, size_of(*p0, *p2))
        }
        _ => false,
    };
    closed.then_some([*p0, *p1, *p2, *p3])
}

/// The rectangle `corners` draw when each side is axis-aligned and
/// the sides alternate; a path otherwise.
fn rect(site: &str, corners: [Point; 4]) -> Result<Classified, String> {
    let [p0, p1, p2, p3] = corners;
    let scale = size_of(p0, p2);
    let horizontal_first = near_value(p0.y, p1.y, scale);
    let sides = [(p0, p1), (p1, p2), (p2, p3), (p3, p0)];
    for (index, (a, b)) in sides.into_iter().enumerate() {
        let horizontal = horizontal_first == (index % 2 == 0);
        let aligned = if horizontal {
            near_value(a.y, b.y, scale)
        } else {
            near_value(a.x, b.x, scale)
        };
        if !aligned {
            return Ok(Classified::Path);
        }
    }
    let extent = [(p2.x - p0.x).abs(), (p2.y - p0.y).abs()];
    if extent[0] <= 0.0 || extent[1] <= 0.0 {
        return Err(format!(
            "the shape at {site} is a rectangle of size {extent:?}; a rectangle needs width and height above zero"
        ));
    }
    let origin = [p0.x.min(p2.x), p0.y.min(p2.y)];
    Ok(Classified::Rect {
        origin,
        size: extent,
    })
}

/// The start, the radii and the first arc's end when `segments` are a
/// move and two unrotated arcs of one radius pair, swept the same way,
/// back to the start, closed or not. The radii and rotations are
/// compared exactly: the source wrote them twice, and a difference is
/// a path.
#[allow(clippy::float_cmp)]
fn circle_arcs(segments: &[Segment]) -> Option<(Point, [f32; 2], Point)> {
    let [
        Segment::Move(p0),
        Segment::Arc {
            radii: r1,
            rotation: rot1,
            sweep: s1,
            to: p1,
            ..
        },
        Segment::Arc {
            radii: r2,
            rotation: rot2,
            sweep: s2,
            to: back,
            ..
        },
        rest @ ..,
    ] = segments
    else {
        return None;
    };
    if !matches!(rest, [] | [Segment::Close]) {
        return None;
    }
    let same = r1 == r2 && *rot1 == 0.0 && *rot2 == 0.0 && s1 == s2;
    let scale = r1[0].abs().max(r1[1].abs());
    (same && near(*back, *p0, scale)).then_some((*p0, *r1, *p1))
}

/// The ellipse two half arcs of `radii` draw from `from` to `to` and
/// back, when the chord is its horizontal or vertical diameter; a path
/// otherwise. The pen corrected the radii: positive, and at least half
/// the chord.
///
/// # Panics
/// When a radius is not above zero: the pen's guarantee broken.
fn ellipse(site: &str, from: Point, [rx, ry]: [f32; 2], to: Point) -> Classified {
    assert!(
        rx > 0.0 && ry > 0.0,
        "the shape at {site} reached the classifier with radii [{rx}, {ry}]; Pen::push makes a zero-radius arc a line"
    );
    let scale = rx.max(ry);
    let horizontal =
        near_value(from.y, to.y, scale) && near_value((from.x - to.x).abs(), 2.0 * rx, scale);
    let vertical =
        near_value(from.x, to.x, scale) && near_value((from.y - to.y).abs(), 2.0 * ry, scale);
    if !(horizontal || vertical) {
        return Classified::Path;
    }
    let centre = [f32::midpoint(from.x, to.x), f32::midpoint(from.y, to.y)];
    Classified::Ellipse {
        centre,
        radii: [rx, ry],
    }
}

/// Whether `a` and `b` are the same point within [`TOLERANCE`] of `scale`.
fn near(a: Point, b: Point, scale: f32) -> bool {
    near_value(a.x, b.x, scale) && near_value(a.y, b.y, scale)
}

fn near_value(a: f32, b: f32, scale: f32) -> bool {
    (a - b).abs() <= TOLERANCE * scale.max(1.0)
}

/// The larger extent between two points, for a tolerance's scale.
fn size_of(a: Point, b: Point) -> f32 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

#[cfg(test)]
#[path = "classify_tests.rs"]
mod tests;

//! A rounded rectangle (one-scene S4b): a path that is exactly the
//! outline SVG gives `<rect rx ry>` (SVG 2, 10.2): a move, then four
//! axis-aligned lines and four quarter arcs alternating, back to the
//! start. Each line lies along the rectangle's edge, as long as the
//! side less both corners. Each arc is an unrotated small arc of one
//! radius pair, spanning exactly its radii, swept the way the path
//! goes round, so that its centre is inside. A stroke ends exactly at
//! the start. Anything else, such as a short arc, corners of different
//! radii or a curved side, stays a path, which the tessellator draws
//! exactly. The radii come out through `geometry::fitted_radii`, the
//! check's own clamp, so what this accepts the check draws (#387
//! review, H1). Each guard has a near miss that only it catches
//! (`classify_round_near_tests.rs`). Tolerances are relative to the
//! shape's own coordinates, with no floor of 1 as `classify_near` has, so
//! a shape under a unit across is held to its own size (#387 review,
//! L1).

use zeno::Point;

use crate::canonical::Paint;
use crate::classify::Classified;
use crate::classify_near::TOLERANCE;
use crate::geometry::fitted_radii;
use crate::segment::Segment;

/// The rounded rectangle `segments` draw under `paint`, if they draw
/// exactly one.
pub(crate) fn round_rect(segments: &[Segment], paint: Paint) -> Option<Classified> {
    let (start, body) = outline(segments, paint)?;
    let points = ends(start, body, paint)?;
    let scale = magnitude_of(&points);
    let (radii, sweep) = arcs(body, scale)?;
    let [lo, hi] = bounds(&points);
    let size = [hi.x - lo.x, hi.y - lo.y];
    let turns = (signed_area(&points) > 0.0) == sweep;
    // a radius the closed-form check refuses stays a path
    let drawable = radii.iter().all(|r| *r >= crate::geometry::MIN_RADIUS);
    let sides = (body.iter().zip(points.windows(2)))
        .all(|(segment, ends)| side(segment, [ends[0], ends[1]], [lo, hi], radii, scale));
    if !(turns && sides && drawable) {
        return None;
    }
    // each side line within the tolerance of its side less both corners
    // holds each radius within half the tolerance of half its side
    let radius = fitted_radii([lo.x, lo.y], size, radii)
        .unwrap_or_else(|reason| panic!("the side check fits the radii: {reason}"));
    Some(Classified::Rect {
        origin: [lo.x, lo.y],
        size,
        radius,
    })
}

/// The start and the eight segments after it, when they end back at
/// the start and are closed (a fill may leave the close out).
fn outline(segments: &[Segment], paint: Paint) -> Option<(Point, &[Segment])> {
    let (Segment::Move(start), rest) = segments.split_first()? else {
        return None;
    };
    let body = match (paint, rest.len()) {
        (_, 9) if rest[8] == Segment::Close => &rest[..8],
        (Paint::Fill(_), 8) => rest,
        _ => return None,
    };
    let alternates = body
        .windows(2)
        .all(|pair| is_arc(&pair[0]) != is_arc(&pair[1]));
    alternates.then_some((*start, body))
}

fn is_arc(segment: &Segment) -> bool {
    matches!(segment, Segment::Arc { .. })
}

/// The start and each segment's end, when every segment is a line or an
/// arc and the last ends at the start: exactly for a stroke, whose close
/// from a point only near the start would be a segment with joins
/// (#387 review, L2; `rect_corners` holds strokes the same way), and
/// within the tolerance for a fill.
fn ends(start: Point, body: &[Segment], paint: Paint) -> Option<Vec<Point>> {
    let mut points = vec![start];
    for segment in body {
        points.push(match *segment {
            Segment::Line(to) | Segment::Arc { to, .. } => to,
            _ => return None,
        });
    }
    let back = points[points.len() - 1];
    let closed = match paint {
        Paint::Stroke { .. } => back == start,
        Paint::Fill(_) => {
            let scale = magnitude_of(&points);
            within(back.x, start.x, scale) && within(back.y, start.y, scale)
        }
    };
    closed.then_some(points)
}

/// The radii and sweep every arc shares, when each is unrotated and
/// small.
#[allow(clippy::float_cmp)] // the source wrote each rotation; 0 is exact
fn arcs(body: &[Segment], scale: f32) -> Option<([f32; 2], bool)> {
    let mut shared = None;
    for segment in body {
        let Segment::Arc {
            radii,
            rotation,
            large,
            sweep,
            ..
        } = *segment
        else {
            continue;
        };
        let (first, first_sweep) = *shared.get_or_insert((radii, sweep));
        let same = within(radii[0], first[0], scale) && within(radii[1], first[1], scale);
        if rotation != 0.0 || large || sweep != first_sweep || !same {
            return None;
        }
    }
    shared
}

/// Whether `segment`, from `ends[0]` to `ends[1]`, is one of the
/// rectangle's sides: a line along an edge of the box `[lo, hi]`, its
/// length the side less both corners, or an arc spanning `radii`.
fn side(
    segment: &Segment,
    ends: [Point; 2],
    [lo, hi]: [Point; 2],
    radii: [f32; 2],
    scale: f32,
) -> bool {
    let [a, b] = ends;
    let span = [(b.x - a.x).abs(), (b.y - a.y).abs()];
    let near_span = |x: f32, y: f32| within(span[0], x, scale) && within(span[1], y, scale);
    if is_arc(segment) {
        return near_span(radii[0], radii[1]);
    }
    let on_edge = |v: f32, low: f32, high: f32| within(v, low, scale) || within(v, high, scale);
    let across = near_span(hi.x - lo.x - 2.0 * radii[0], 0.0) && on_edge(a.y, lo.y, hi.y);
    let down = near_span(0.0, hi.y - lo.y - 2.0 * radii[1]) && on_edge(a.x, lo.x, hi.x);
    across || down
}

/// Whether `a` and `b` are the same within [`TOLERANCE`] of `scale`.
fn within(a: f32, b: f32, scale: f32) -> bool {
    (a - b).abs() <= TOLERANCE * scale
}

/// The largest coordinate magnitude among `points`, with no floor.
fn magnitude_of(points: &[Point]) -> f32 {
    points
        .iter()
        .map(|p| p.x.abs().max(p.y.abs()))
        .fold(0.0, f32::max)
}

/// The low and high corners of the box holding `points`.
fn bounds(points: &[Point]) -> [Point; 2] {
    let fold = |pick: fn(f32, f32) -> f32, start: f32| {
        let x = points.iter().map(|p| p.x).fold(start, pick);
        let y = points.iter().map(|p| p.y).fold(start, pick);
        Point::new(x, y)
    };
    [
        fold(f32::min, f32::INFINITY),
        fold(f32::max, f32::NEG_INFINITY),
    ]
}

/// Twice the signed area of the polygon through `points`: positive when
/// it goes clockwise on screen (y down), the way SVG's sweep flag 1
/// turns.
fn signed_area(points: &[Point]) -> f32 {
    points
        .windows(2)
        .map(|pair| pair[0].x * pair[1].y - pair[1].x * pair[0].y)
        .sum()
}

#[cfg(test)]
#[path = "classify_round_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "classify_round_near_tests.rs"]
mod near_tests;

#[cfg(test)]
#[path = "classify_round_stroke_tests.rs"]
mod stroke_tests;

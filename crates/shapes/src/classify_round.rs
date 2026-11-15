//! A rounded rectangle (one-scene S4b): a path that is exactly the
//! outline SVG gives `<rect rx ry>` (SVG 2, 10.2): a move, then four
//! axis-aligned lines and four quarter arcs alternating, back to the
//! start. Each line lies along the rectangle's edge, as long as the
//! side less both corners. Each arc is an unrotated small arc of one
//! radius pair, spanning exactly its radii, swept the way the path
//! goes round, so that its centre is inside. Anything else, such as a
//! short arc, corners of different radii or an extra segment, stays a
//! path, which the tessellator draws exactly.

use zeno::Point;

use crate::canonical::Paint;
use crate::classify::Classified;
use crate::classify_near::{near, near_value, scale_of};
use crate::segment::Segment;

/// The rounded rectangle `segments` draw under `paint`, if they draw
/// exactly one.
pub(crate) fn round_rect(segments: &[Segment], paint: Paint) -> Option<Classified> {
    let (start, body) = outline(segments, paint)?;
    let points = ends(start, body)?;
    let scale = scale_of(&points);
    let (radii, sweep) = arcs(body, scale)?;
    let [lo, hi] = bounds(&points);
    let size = [hi.x - lo.x, hi.y - lo.y];
    let fits = (0..2).all(|i| 2.0 * radii[i] <= size[i] + scale * crate::classify_near::TOLERANCE);
    let turns = (signed_area(&points) > 0.0) == sweep;
    // a radius the closed-form check refuses stays a path
    let drawable = radii.iter().all(|r| *r >= crate::geometry::MIN_RADIUS);
    let sides = (body.iter().zip(points.windows(2)))
        .all(|(segment, ends)| side(segment, [ends[0], ends[1]], [lo, hi], radii, scale));
    (fits && turns && sides && drawable).then_some(Classified::Rect {
        origin: [lo.x, lo.y],
        size,
        radius: radii,
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
/// arc and the last ends at the start.
fn ends(start: Point, body: &[Segment]) -> Option<Vec<Point>> {
    let mut points = vec![start];
    for segment in body {
        points.push(match *segment {
            Segment::Line(to) | Segment::Arc { to, .. } => to,
            _ => return None,
        });
    }
    let back = points[points.len() - 1];
    near(back, start, scale_of(&points)).then_some(points)
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
        let same = near_value(radii[0], first[0], scale) && near_value(radii[1], first[1], scale);
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
    let near_span = |x: f32, y: f32| near_value(span[0], x, scale) && near_value(span[1], y, scale);
    if is_arc(segment) {
        return near_span(radii[0], radii[1]);
    }
    let on_edge =
        |v: f32, low: f32, high: f32| near_value(v, low, scale) || near_value(v, high, scale);
    let across = near_span(hi.x - lo.x - 2.0 * radii[0], 0.0) && on_edge(a.y, lo.y, hi.y);
    let down = near_span(0.0, hi.y - lo.y - 2.0 * radii[1]) && on_edge(a.x, lo.x, hi.x);
    across || down
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

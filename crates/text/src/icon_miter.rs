//! Miter joins sharper than 90 degrees. zeno 0.3.3 bevels every join
//! whose normals are more than 90 degrees apart whatever the miter limit
//! (dfrg/zeno#29, docs/icons.md), where SVG miters up to the limit of 4. For
//! those joins this adds the missing wedge, pivot to the two outer
//! corners and the miter tip, as polygons to fill beside the stroke.

use zeno::{Command, Point, Vector};

/// SVG's default `stroke-miterlimit`.
const MITER_LIMIT: f32 = 4.0;

/// Fill commands for the wedges `commands` stroked `width` wide lack.
pub(crate) fn missing_wedges(commands: &[Command], width: f32) -> Vec<Command> {
    let mut wedges = Vec::new();
    for (pivot, incoming, outgoing) in joins(commands) {
        push_wedge(&mut wedges, pivot, incoming, outgoing, width / 2.0);
    }
    wedges
}

fn push_wedge(wedges: &mut Vec<Command>, pivot: Point, d1: Vector, d2: Vector, radius: f32) {
    let dot = d1.dot(d2);
    let sin_half = f32::midpoint(1.0, dot).sqrt();
    if dot >= 0.0 || sin_half * MITER_LIMIT < 1.0 {
        return;
    }
    let outward = (d1 - d2).normalize();
    let outer = |d: Vector| {
        let normal = Vector::new(-d.y, d.x);
        if normal.dot(outward) < 0.0 {
            normal * -1.0
        } else {
            normal
        }
    };
    wedges.push(Command::MoveTo(pivot));
    wedges.push(Command::LineTo(pivot + outer(d1) * radius));
    wedges.push(Command::LineTo(pivot + outward * (radius / sin_half)));
    wedges.push(Command::LineTo(pivot + outer(d2) * radius));
    wedges.push(Command::Close);
}

/// Each join: its point and the unit directions in and out of it.
fn joins(commands: &[Command]) -> Vec<(Point, Vector, Vector)> {
    let mut joins = Vec::new();
    let (mut start, mut current) = (Point::ZERO, Point::ZERO);
    let (mut first, mut last): (Option<Vector>, Option<Vector>) = (None, None);
    for command in commands {
        let (from, to, end) = match *command {
            Command::MoveTo(p) => {
                (start, current, first, last) = (p, p, None, None);
                continue;
            }
            Command::LineTo(p) => (p - current, p - current, p),
            Command::CurveTo(a, b, p) => {
                (tangent(&[a, b, p], current), tangent(&[b, a], p) * -1.0, p)
            }
            Command::QuadTo(a, p) => (tangent(&[a, p], current), tangent(&[a], p) * -1.0, p),
            Command::Close => {
                if let (Some(d1), Some(d2)) = (last, first) {
                    let close = start - current;
                    let closing = (close.length() > 1e-6).then(|| close.normalize());
                    push_join(&mut joins, current, last, closing);
                    push_join(&mut joins, start, closing.or(Some(d1)), Some(d2));
                }
                (current, first, last) = (start, None, None);
                continue;
            }
        };
        if to.length() > 1e-6 || from.length() > 1e-6 {
            let (from, to) = (from.normalize(), to.normalize());
            push_join(&mut joins, current, last, Some(from));
            first = first.or(Some(from));
            last = Some(to);
        }
        current = end;
    }
    joins
}

fn push_join(
    joins: &mut Vec<(Point, Vector, Vector)>,
    at: Point,
    d1: Option<Vector>,
    d2: Option<Vector>,
) {
    if let (Some(d1), Some(d2)) = (d1, d2) {
        joins.push((at, d1, d2));
    }
}

/// The direction from `origin` to the first of `points` apart from it.
fn tangent(points: &[Point], origin: Point) -> Vector {
    points
        .iter()
        .map(|&p| p - origin)
        .find(|v| v.length() > 1e-6)
        .unwrap_or(Vector::ZERO)
}

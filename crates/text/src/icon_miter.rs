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
    // A reversal on a diagonal can give a dot just under -1.
    let sin_half = f32::midpoint(1.0, dot).max(0.0).sqrt();
    if dot >= 0.0 || sin_half * MITER_LIMIT < 1.0 {
        return;
    }
    let outward = (d1 - d2).normalize();
    let points = [
        pivot,
        pivot + outer_normal(d1, outward) * radius,
        pivot + outward * (radius / sin_half),
        pivot + outer_normal(d2, outward) * radius,
    ];
    // zeno panics on a non-finite point in debug builds.
    if points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return;
    }
    wedges.push(Command::MoveTo(points[0]));
    wedges.extend(points[1..].iter().map(|&p| Command::LineTo(p)));
    wedges.push(Command::Close);
}

/// The unit normal of `d` on the outer side of the join.
fn outer_normal(d: Vector, outward: Vector) -> Vector {
    let normal = Vector::new(-d.y, d.x);
    if normal.dot(outward) < 0.0 {
        normal * -1.0
    } else {
        normal
    }
}

/// Each join: its point and the unit directions in and out of it.
type Join = (Point, Vector, Vector);

fn joins(commands: &[Command]) -> Vec<Join> {
    let mut walk = Walk::default();
    for command in commands {
        walk.step(command);
    }
    walk.joins
}

/// The state of a walk along the commands' subpaths.
#[derive(Default)]
struct Walk {
    start: Point,
    current: Point,
    first: Option<Vector>,
    last: Option<Vector>,
    joins: Vec<Join>,
}

impl Walk {
    fn step(&mut self, command: &Command) {
        match *command {
            Command::MoveTo(p) => self.begin(p),
            Command::LineTo(p) => self.segment(p - self.current, p - self.current, p),
            Command::CurveTo(a, b, p) => {
                let out = tangent(&[a, b, p], self.current);
                self.segment(out, tangent(&[b, a], p) * -1.0, p);
            }
            Command::QuadTo(a, p) => {
                let out = tangent(&[a, p], self.current);
                self.segment(out, tangent(&[a], p) * -1.0, p);
            }
            Command::Close => self.close(),
        }
    }

    fn begin(&mut self, at: Point) {
        (self.start, self.current, self.first, self.last) = (at, at, None, None);
    }

    /// A segment leaving in direction `from` and arriving in direction `to`.
    fn segment(&mut self, from: Vector, to: Vector, end: Point) {
        if from.length() > 1e-6 {
            let from = from.normalize();
            // A curve ending on its own control points arrives as it left.
            let to = if to.length() > 1e-6 {
                to.normalize()
            } else {
                from
            };
            push_join(&mut self.joins, self.current, self.last, Some(from));
            self.first = self.first.or(Some(from));
            self.last = Some(to);
        }
        self.current = end;
    }

    fn close(&mut self) {
        if let (Some(d1), Some(d2)) = (self.last, self.first) {
            let close = self.start - self.current;
            let closing = (close.length() > 1e-6).then(|| close.normalize());
            push_join(&mut self.joins, self.current, self.last, closing);
            push_join(&mut self.joins, self.start, closing.or(Some(d1)), Some(d2));
        }
        (self.current, self.first, self.last) = (self.start, None, None);
    }
}

fn push_join(joins: &mut Vec<Join>, at: Point, d1: Option<Vector>, d2: Option<Vector>) {
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

#[cfg(test)]
#[path = "icon_miter_tests.rs"]
mod tests;

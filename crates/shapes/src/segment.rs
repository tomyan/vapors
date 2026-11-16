//! A parsed path as the source wrote it, made absolute, with SVG's arc
//! corrections applied: the form the canonical bytes, the classifier
//! and the tessellator read. The icon route reads zeno's commands
//! instead, a plain mapping of these by [`to_zeno`], where an arc
//! becomes zeno's curves.

use zeno::{Angle, ArcSize, ArcSweep, Command, PathBuilder, Point};

/// One command of SVG path data, absolute, in viewBox units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Segment {
    /// `M`: a new subpath at the point.
    Move(Point),
    /// `L` (and `H`, `V`): a line to the point.
    Line(Point),
    /// `Q`: a quadratic curve through the control point to the end.
    Quad(Point, Point),
    /// `C`: a cubic curve through two control points to the end.
    Cubic(Point, Point, Point),
    /// `A`: an elliptical arc to `to`, as the [`Pen`] corrected it (SVG
    /// 2, 9.5.1 and F.6.6): the radii are positive and large enough for
    /// the chord; a zero radius became a [`Segment::Line`] and an arc to
    /// the current point was dropped.
    Arc {
        /// `rx`, `ry`, both above zero.
        radii: [f32; 2],
        /// The x-axis rotation, degrees.
        rotation: f32,
        /// The large-arc flag.
        large: bool,
        /// The sweep flag.
        sweep: bool,
        /// The end point.
        to: Point,
    },
    /// `Z`: a line back to the subpath's start.
    Close,
}

/// The segments so far and where the pen is: the current point, which
/// relative commands add to, and the subpath's start, where a close
/// returns it. Every segment goes through [`Pen::push`], so the arc
/// corrections are applied once, here, for every reader.
#[derive(Debug, Default)]
pub struct Pen {
    segments: Vec<Segment>,
    current: Point,
    start: Point,
}

impl Pen {
    /// Adds `segment`, correcting an arc as SVG says, and moves the pen.
    pub fn push(&mut self, segment: Segment) {
        let segment = match segment {
            Segment::Arc {
                radii,
                rotation,
                large,
                sweep,
                to,
            } => match corrected_arc(self.current, radii, rotation, to) {
                Corrected::Nothing => return,
                Corrected::Line => Segment::Line(to),
                Corrected::Arc(radii) => Segment::Arc {
                    radii,
                    rotation,
                    large,
                    sweep,
                    to,
                },
            },
            other => other,
        };
        self.current = match segment {
            Segment::Move(p) => {
                self.start = p;
                p
            }
            Segment::Line(p) | Segment::Quad(_, p) | Segment::Cubic(_, _, p) => p,
            Segment::Arc { to, .. } => to,
            Segment::Close => self.start,
        };
        self.segments.push(segment);
    }

    /// Where the pen is.
    #[must_use]
    pub const fn current(&self) -> Point {
        self.current
    }

    /// The segments so far.
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// The segments.
    #[must_use]
    pub fn into_segments(self) -> Vec<Segment> {
        self.segments
    }
}

/// The first value of `segment` that is not a finite number, if any:
/// a relative command can add two finite coordinates to an infinite
/// one, and the parser refuses it where it is made.
#[must_use]
pub fn non_finite(segment: &Segment) -> Option<f32> {
    let values: Vec<f32> = match *segment {
        Segment::Move(p) | Segment::Line(p) => vec![p.x, p.y],
        Segment::Quad(c, p) => vec![c.x, c.y, p.x, p.y],
        Segment::Cubic(a, b, p) => vec![a.x, a.y, b.x, b.y, p.x, p.y],
        Segment::Arc {
            radii,
            rotation,
            to,
            ..
        } => vec![radii[0], radii[1], rotation, to.x, to.y],
        Segment::Close => vec![],
    };
    values.into_iter().find(|v| !v.is_finite())
}

/// What an arc becomes once corrected.
enum Corrected {
    /// The end point is the current point: the arc is omitted.
    Nothing,
    /// A radius is zero: a straight line to the end point.
    Line,
    /// An arc with these radii, taken absolute and scaled up when too
    /// small for the chord.
    Arc([f32; 2]),
}

/// SVG 2's arc corrections (9.5.1, F.6.6), from `from` to `to`.
fn corrected_arc(from: Point, [rx, ry]: [f32; 2], rotation: f32, to: Point) -> Corrected {
    if from == to {
        return Corrected::Nothing;
    }
    let (rx, ry) = (rx.abs(), ry.abs());
    if rx == 0.0 || ry == 0.0 {
        return Corrected::Line;
    }
    let (sin, cos) = rotation.to_radians().sin_cos();
    let (dx, dy) = ((from.x - to.x) / 2.0, (from.y - to.y) / 2.0);
    let (x1, y1) = (cos * dx + sin * dy, -sin * dx + cos * dy);
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lambda > 1.0 {
        let scale = lambda.sqrt();
        return Corrected::Arc([rx * scale, ry * scale]);
    }
    Corrected::Arc([rx, ry])
}

/// `segments` as zeno's commands, one each; an arc as zeno's curves.
#[must_use]
pub fn to_zeno(segments: &[Segment]) -> Vec<Command> {
    let mut out = Vec::with_capacity(segments.len());
    for segment in segments {
        match *segment {
            Segment::Move(p) => out.move_to(p),
            Segment::Line(p) => out.line_to(p),
            Segment::Quad(c, p) => out.quad_to(c, p),
            Segment::Cubic(a, b, p) => out.curve_to(a, b, p),
            Segment::Arc {
                radii: [rx, ry],
                rotation,
                large,
                sweep,
                to,
            } => out.arc_to(
                rx,
                ry,
                Angle::from_degrees(rotation),
                if large {
                    ArcSize::Large
                } else {
                    ArcSize::Small
                },
                if sweep {
                    ArcSweep::Positive
                } else {
                    ArcSweep::Negative
                },
                to,
            ),
            Segment::Close => out.close(),
        };
    }
    out
}

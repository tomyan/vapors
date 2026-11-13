//! A parsed path as the source wrote it, made absolute: the form the
//! canonical bytes, the classifier and the tessellator read. The icon
//! route reads zeno's commands instead, derived from these by
//! [`to_zeno`], where an arc becomes zeno's curves.

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
    /// `A`: an elliptical arc to `to`.
    Arc {
        /// `rx`, `ry`; a zero radius draws a line (SVG 2, 9.5.1).
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

/// `segments` as zeno's commands. A zero-radius arc is a straight
/// line, which zeno would turn into NaN; zeno takes negative radii as
/// absolute, scales radii too small up, and skips an end point equal
/// to the current point, as SVG says.
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
                radii,
                rotation,
                large,
                sweep,
                to,
            } => arc(&mut out, radii, rotation, large, sweep, to),
            Segment::Close => out.close(),
        };
    }
    out
}

fn arc(
    out: &mut Vec<Command>,
    [rx, ry]: [f32; 2],
    rotation: f32,
    large: bool,
    sweep: bool,
    to: Point,
) -> &mut Vec<Command> {
    if rx == 0.0 || ry == 0.0 {
        return out.line_to(to);
    }
    let size = if large {
        ArcSize::Large
    } else {
        ArcSize::Small
    };
    let sweep = if sweep {
        ArcSweep::Positive
    } else {
        ArcSweep::Negative
    };
    out.arc_to(rx, ry, Angle::from_degrees(rotation), size, sweep, to)
}

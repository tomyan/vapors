//! A path's segments as a lyon path for the tessellator, each arc as
//! cubics, every segment starting where SVG says the pen is.

use lyon_tessellation::geom::{CubicBezierSegment, SvgArc};
use lyon_tessellation::math::{Angle, Point as LyonPoint, point, vector};
use lyon_tessellation::path::builder::WithSvg;
use lyon_tessellation::path::path::BuilderImpl;
use lyon_tessellation::path::{ArcFlags, Path};

use crate::segment::Segment;

/// `segments` as a lyon path, each arc as cubics (within 2.7e-4 of
/// its radius per 90 degrees; lyon's own `arc_to` goes through 45
/// degree quadratics, 0.31% of the radius out, which no tolerance
/// bounds). An arc starts at the builder's current position, which a
/// close moves to the subpath's start, as SVG's pen does.
pub(crate) fn lyon_path(segments: &[Segment]) -> Path {
    let mut builder = Path::svg_builder();
    for segment in segments {
        match *segment {
            Segment::Move(p) => {
                builder.move_to(lyon(p));
            }
            Segment::Line(p) => {
                builder.line_to(lyon(p));
            }
            Segment::Quad(c, p) => {
                builder.quadratic_bezier_to(lyon(c), lyon(p));
            }
            Segment::Cubic(a, b, p) => {
                builder.cubic_bezier_to(lyon(a), lyon(b), lyon(p));
            }
            Segment::Arc {
                radii,
                rotation,
                large,
                sweep,
                to,
            } => arc_as_cubics(&mut builder, radii, rotation, [large, sweep], to),
            Segment::Close => builder.close(),
        }
    }
    builder.build()
}

fn lyon(p: zeno::Point) -> LyonPoint {
    point(p.x, p.y)
}

fn arc_as_cubics(
    builder: &mut WithSvg<BuilderImpl>,
    radii: [f32; 2],
    rotation: f32,
    [large, sweep]: [bool; 2],
    to: zeno::Point,
) {
    assert!(
        radii[0] > 0.0 && radii[1] > 0.0,
        "an arc with radii {radii:?} reached the tessellator; Pen::push makes a zero-radius arc a line"
    );
    let arc = SvgArc {
        from: builder.current_position(),
        to: lyon(to),
        radii: vector(radii[0], radii[1]),
        x_rotation: Angle::degrees(rotation),
        flags: ArcFlags {
            large_arc: large,
            sweep,
        },
    };
    arc.for_each_cubic_bezier(&mut |c: &CubicBezierSegment<f32>| {
        builder.cubic_bezier_to(c.ctrl1, c.ctrl2, c.to);
    });
}

#[cfg(test)]
#[path = "tessellate_path_tests.rs"]
mod tests;

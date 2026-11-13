//! The tessellator (docs/plans/one-scene.md 1.2): a path that is not a
//! closed form becomes triangles through lyon, fills under their rule
//! and strokes with SVG's default miter joins (limit 4) and butt caps.
//! `mesh_edges` then gives each triangle its own vertices with the
//! distances, normals and shifts the fragment shader takes coverage
//! from after the transform.

use lyon_tessellation::geom::{CubicBezierSegment, SvgArc};
use lyon_tessellation::geometry_builder::{BuffersBuilder, VertexBuffers};
use lyon_tessellation::math::{Angle, Point as LyonPoint, point, vector};
use lyon_tessellation::path::{ArcFlags, LineCap, LineJoin, Path};
use lyon_tessellation::{
    FillOptions, FillRule as LyonRule, FillTessellator, FillVertex, StrokeOptions,
    StrokeTessellator, StrokeVertex,
};

use crate::canonical::{FillRule, Paint};
use crate::mesh::MeshVertex;
use crate::mesh_edges::triangles_with_edges;
use crate::path::IconPath;
use crate::segment::Segment;

/// Triangles with their edge distances, three vertices each, and the
/// tolerance they were flattened at.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    /// The triangles, in no particular order.
    pub triangles: Vec<[MeshVertex; 3]>,
    /// The flattening tolerance, viewBox units: a draw whose scale
    /// would show a coarser flattening than this tessellates again
    /// (svg-support D1, the per-entry record).
    pub tolerance: f32,
}

type Buffers = VertexBuffers<[f32; 2], u32>;

/// Tessellates `path` under `paint`, flattening curves (arcs as
/// cubics first) to within `tolerance` viewBox units (chosen for the
/// largest scale the shape is drawn at; S3b measures it); `site`
/// names the shape in a refusal.
///
/// # Errors
/// A tolerance or stroke width that is not a finite number above
/// zero; a tessellation lyon refuses (naming its error); a result
/// with no triangle of any area, which SVG draws as nothing and
/// vapors refuses rather than keeping an empty entry.
pub fn tessellate(
    site: &str,
    path: &IconPath,
    paint: Paint,
    tolerance: f32,
) -> Result<Mesh, String> {
    check_positive(site, "tolerance", tolerance)?;
    if let Paint::Stroke { width } = paint {
        check_positive(site, "stroke-width", width)?;
    }
    let lyon_path = lyon_path(path.segments());
    let mut buffers = Buffers::new();
    match paint {
        Paint::Fill(rule) => fill(&lyon_path, rule, tolerance, &mut buffers),
        Paint::Stroke { width } => stroke(&lyon_path, width, tolerance, &mut buffers),
    }
    .map_err(|error| format!("the shape at {site} cannot be tessellated: {error}"))?;
    let triangles = triangles_with_edges(&buffers.vertices, &buffers.indices);
    if triangles.is_empty() {
        return Err(no_area(site, paint));
    }
    Ok(Mesh {
        triangles,
        tolerance,
    })
}

fn check_positive(site: &str, what: &str, value: f32) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        return Ok(());
    }
    Err(format!(
        "the shape at {site} has {what} {value}, which is not a finite number above zero; write a positive {what}"
    ))
}

fn no_area(site: &str, paint: Paint) -> String {
    let how = match paint {
        Paint::Fill(_) => "filled",
        Paint::Stroke { .. } => "stroked",
    };
    format!(
        "the shape at {site} has no area when {how}; nothing would be drawn, so remove it or give it area"
    )
}

fn fill(path: &Path, rule: FillRule, tolerance: f32, out: &mut Buffers) -> Result<(), String> {
    let rule = match rule {
        FillRule::NonZero => LyonRule::NonZero,
        FillRule::EvenOdd => LyonRule::EvenOdd,
    };
    let options = FillOptions::tolerance(tolerance).with_fill_rule(rule);
    let mut builder = BuffersBuilder::new(out, |v: FillVertex| v.position().to_array());
    FillTessellator::new()
        .tessellate_path(path, &options, &mut builder)
        .map_err(|e| e.to_string())
}

fn stroke(path: &Path, width: f32, tolerance: f32, out: &mut Buffers) -> Result<(), String> {
    let options = StrokeOptions::tolerance(tolerance)
        .with_line_width(width)
        .with_line_join(LineJoin::Miter)
        .with_miter_limit(4.0)
        .with_line_cap(LineCap::Butt);
    let mut builder = BuffersBuilder::new(out, |v: StrokeVertex| v.position().to_array());
    StrokeTessellator::new()
        .tessellate_path(path, &options, &mut builder)
        .map_err(|e| e.to_string())
}

/// `segments` as a lyon path, each arc as cubics (within 2.7e-4 of
/// its radius per 90 degrees; lyon's own `arc_to` goes through 45
/// degree quadratics, 0.31% of the radius out, which no tolerance
/// bounds).
fn lyon_path(segments: &[Segment]) -> Path {
    let mut builder = Path::svg_builder();
    let mut current = point(0.0, 0.0);
    for segment in segments {
        current = match *segment {
            Segment::Move(p) => {
                builder.move_to(point(p.x, p.y));
                point(p.x, p.y)
            }
            Segment::Line(p) => {
                builder.line_to(point(p.x, p.y));
                point(p.x, p.y)
            }
            Segment::Quad(c, p) => {
                builder.quadratic_bezier_to(point(c.x, c.y), point(p.x, p.y));
                point(p.x, p.y)
            }
            Segment::Cubic(a, b, p) => {
                builder.cubic_bezier_to(point(a.x, a.y), point(b.x, b.y), point(p.x, p.y));
                point(p.x, p.y)
            }
            Segment::Arc {
                radii,
                rotation,
                large,
                sweep,
                to,
            } => arc_as_cubics(&mut builder, current, radii, rotation, [large, sweep], to),
            Segment::Close => {
                builder.close();
                current
            }
        };
    }
    builder.build()
}

fn arc_as_cubics(
    builder: &mut lyon_tessellation::path::builder::WithSvg<
        lyon_tessellation::path::path::BuilderImpl,
    >,
    from: LyonPoint,
    radii: [f32; 2],
    rotation: f32,
    [large, sweep]: [bool; 2],
    to: zeno::Point,
) -> LyonPoint {
    assert!(
        radii[0] > 0.0 && radii[1] > 0.0,
        "an arc with radii {radii:?} reached the tessellator; Pen::push makes a zero-radius arc a line"
    );
    let arc = SvgArc {
        from,
        to: point(to.x, to.y),
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
    point(to.x, to.y)
}

#[cfg(test)]
#[path = "tessellate_tests.rs"]
mod tests;

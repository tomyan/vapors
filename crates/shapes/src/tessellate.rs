//! The tessellator (docs/plans/one-scene.md 1.2): a path that is not a
//! closed form becomes triangles through lyon, fills under their rule
//! and strokes with SVG's default miter joins (limit 4) and butt caps.
//! Each triangle's vertices then carry their distances to its edges
//! (`mesh_edges`), so the fragment shader takes coverage from the
//! outline after the transform.

use lyon_tessellation::geometry_builder::{BuffersBuilder, VertexBuffers};
use lyon_tessellation::math::{Angle, point, vector};
use lyon_tessellation::path::traits::SvgPathBuilder;
use lyon_tessellation::path::{ArcFlags, LineCap, LineJoin, Path};
use lyon_tessellation::{
    FillOptions, FillRule as LyonRule, FillTessellator, FillVertex, StrokeOptions,
    StrokeTessellator, StrokeVertex,
};

use crate::canonical::{FillRule, Paint};
use crate::geometry::MeshVertex;
use crate::mesh_edges::triangles_with_edges;
use crate::path::IconPath;
use crate::segment::Segment;

/// Triangles with their edge distances, three vertices each.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    /// The triangles, in no particular order.
    pub triangles: Vec<[MeshVertex; 3]>,
}

/// Tessellates `path` under `paint`, flattening curves to within
/// `tolerance` viewBox units (chosen for the largest scale the shape
/// is drawn at; S3b measures it); `site` names the shape in a refusal.
///
/// # Errors
/// A tessellation lyon refuses (naming its error), or a result with no
/// triangle of any area: a fill without area or a stroke of no length,
/// which SVG draws as nothing and vapors refuses rather than keeping an
/// empty entry.
pub fn tessellate(
    site: &str,
    path: &IconPath,
    paint: Paint,
    tolerance: f32,
) -> Result<Mesh, String> {
    let lyon_path = lyon_path(path.segments());
    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    let result = match paint {
        Paint::Fill(rule) => fill(&lyon_path, rule, tolerance, &mut buffers),
        Paint::Stroke { width } => stroke(&lyon_path, width, tolerance, &mut buffers),
    };
    if let Err(error) = result {
        return Err(format!(
            "the shape at {site} cannot be tessellated: {error}"
        ));
    }
    let triangles = triangles_with_edges(&buffers.vertices, &buffers.indices);
    if triangles.is_empty() {
        return Err(format!(
            "the shape at {site} has no area when {}; nothing would be drawn, so remove it or give it area",
            match paint {
                Paint::Fill(_) => "filled",
                Paint::Stroke { .. } => "stroked",
            }
        ));
    }
    Ok(Mesh { triangles })
}

fn fill(
    path: &Path,
    rule: FillRule,
    tolerance: f32,
    out: &mut VertexBuffers<[f32; 2], u32>,
) -> Result<(), String> {
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

fn stroke(
    path: &Path,
    width: f32,
    tolerance: f32,
    out: &mut VertexBuffers<[f32; 2], u32>,
) -> Result<(), String> {
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

/// `segments` as a lyon path. A zero-radius arc is a line, as
/// `segment::to_zeno` draws it (SVG 2, 9.5.1).
fn lyon_path(segments: &[Segment]) -> Path {
    let mut builder = Path::svg_builder();
    for segment in segments {
        match *segment {
            Segment::Move(p) => {
                builder.move_to(point(p.x, p.y));
            }
            Segment::Line(p) => {
                builder.line_to(point(p.x, p.y));
            }
            Segment::Quad(c, p) => {
                builder.quadratic_bezier_to(point(c.x, c.y), point(p.x, p.y));
            }
            Segment::Cubic(a, b, p) => {
                builder.cubic_bezier_to(point(a.x, a.y), point(b.x, b.y), point(p.x, p.y));
            }
            Segment::Arc {
                radii,
                rotation,
                large,
                sweep,
                to,
            } if radii[0] != 0.0 && radii[1] != 0.0 => builder.arc_to(
                vector(radii[0], radii[1]),
                Angle::degrees(rotation),
                ArcFlags {
                    large_arc: large,
                    sweep,
                },
                point(to.x, to.y),
            ),
            Segment::Arc { to, .. } => {
                builder.line_to(point(to.x, to.y));
            }
            Segment::Close => builder.close(),
        }
    }
    builder.build()
}

#[cfg(test)]
#[path = "tessellate_tests.rs"]
mod tests;

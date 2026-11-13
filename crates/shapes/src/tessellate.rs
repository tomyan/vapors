//! The tessellator (docs/plans/one-scene.md 1.2): a path that is not a
//! closed form becomes triangles through lyon, fills under their rule
//! and strokes with SVG's default miter joins (limit 4) and butt caps.
//! `mesh_edges` then gives each triangle its own vertices with the
//! distances, normals and shifts the fragment shader takes coverage
//! from after the transform.

use lyon_tessellation::geometry_builder::{BuffersBuilder, VertexBuffers};
use lyon_tessellation::path::{LineCap, LineJoin, Path};
use lyon_tessellation::{
    FillOptions, FillRule as LyonRule, FillTessellator, FillVertex, StrokeOptions,
    StrokeTessellator, StrokeVertex,
};

use crate::canonical::{FillRule, Paint};
use crate::mesh::MeshVertex;
use crate::mesh_edges::triangles_with_edges;
use crate::path::IconPath;
use crate::tessellate_path::lyon_path;

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

#[cfg(test)]
#[path = "tessellate_tests.rs"]
mod tests;

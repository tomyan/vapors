//! A tessellated shape's triangles, each with its own three vertices
//! (docs/plans/one-scene.md 1.2), and the check that every number in
//! them is what the rule says: the fragment shader takes coverage from
//! these numbers after the transform, so a wrong one draws a seam or
//! a hard edge rather than failing.
//!
//! The rule, by edge. Edge `i` of a triangle is the one opposite
//! corner `i`. For an outline edge, the two corners on it carry
//! distance 0 and the third its altitude, so the interpolated value is
//! a fragment's signed distance to the edge's line. For an edge two
//! triangles share, all three corners carry [`FAR_EDGE`], so the value
//! is constant and never the nearest. The vertex shader moves each
//! vertex by `h * normal`, `h` half a device pixel in viewBox units,
//! and takes `h * shift[i]` off `edges[i]`, so the ramp from 0 to 1
//! coverage straddles the true outline (Tom, 2026-11-13, "option 2").
//! `shift[i]` is `normal` projected on edge `i`'s outward normal: 1 on
//! a corner's own outline edges (the miter offset makes it so), the
//! projection on the opposite edge, 0 for a far edge, which stays far.

/// The distance written for an edge two triangles share: finite (an
/// infinite marker makes `fwidth` NaN in the fragment shader) and past
/// any viewBox span in use (icons are 16 to 24 units, scene decals
/// hundreds); a span near it needs a larger value and a
/// [`crate::TABLE_FORMAT`] bump.
pub const FAR_EDGE: f32 = 1.0e6;

/// A vertex of one triangle: 40 bytes, in field order, so the table's
/// triangles upload as they are. The mesh pipeline (S3a2) derives its
/// layout from the offsets asserted below.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshVertex {
    /// Its position in viewBox units.
    pub position: [f32; 2],
    /// The direction it moves out of the shape, per half device pixel:
    /// the miter offset of its outline edges, zero for a vertex inside.
    pub normal: [f32; 2],
    /// Its distance to each of the triangle's three edges, as the
    /// module doc says.
    pub edges: [f32; 3],
    /// What `h * normal` takes off each distance, per unit of `h`.
    pub shift: [f32; 3],
}

const _: () = assert!(size_of::<MeshVertex>() == 40);
const _: () = assert!(std::mem::offset_of!(MeshVertex, normal) == 8);
const _: () = assert!(std::mem::offset_of!(MeshVertex, edges) == 16);
const _: () = assert!(std::mem::offset_of!(MeshVertex, shift) == 28);

/// How far a distance may differ from the one the positions give,
/// relative to the altitude, for f32 arithmetic in two places.
const SLACK: f32 = 1e-4;

/// Why `triangles` cannot be drawn, if they cannot: no triangle; a
/// value that is not finite; a triangle without area; an edge far on
/// some corners and not others; a distance that is neither zero on its
/// corners, the altitude opposite, nor far; a shift that is not the
/// normal's projection on the edge, or not zero on a far edge; a
/// corner on an outline edge whose normal does not move it out across
/// that edge.
///
/// # Errors
/// The reason, naming the triangle, the corner and the field.
pub fn check_mesh(triangles: &[[MeshVertex; 3]]) -> Result<(), String> {
    if triangles.is_empty() {
        return Err("a mesh with no triangles draws nothing".to_owned());
    }
    for (t, triangle) in triangles.iter().enumerate() {
        check_triangle(triangle).map_err(|reason| format!("triangle {t} {reason}"))?;
    }
    Ok(())
}

fn check_triangle(triangle: &[MeshVertex; 3]) -> Result<(), String> {
    for (c, vertex) in triangle.iter().enumerate() {
        check_finite(c, vertex)?;
    }
    let points = triangle.map(|v| v.position);
    let area2 = twice_area(points);
    if area2 == 0.0 {
        return Err(format!("has no area: corners {points:?}"));
    }
    for edge in 0..3 {
        check_edge(triangle, points, area2.abs(), edge)?;
    }
    Ok(())
}

fn check_finite(c: usize, vertex: &MeshVertex) -> Result<(), String> {
    let fields: [(&str, &[f32]); 4] = [
        ("position", &vertex.position),
        ("normal", &vertex.normal),
        ("edges", &vertex.edges),
        ("shift", &vertex.shift),
    ];
    for (name, values) in fields {
        if !values.iter().all(|v| v.is_finite()) {
            return Err(format!("corner {c} {name} {values:?} is not finite"));
        }
    }
    Ok(())
}

/// Edge `edge` of the triangle: far on every corner with shift 0, or
/// on the outline with each corner's distance and shift from the
/// positions.
fn check_edge(
    triangle: &[MeshVertex; 3],
    points: [[f32; 2]; 3],
    area2: f32,
    edge: usize,
) -> Result<(), String> {
    let far: Vec<bool> = triangle.iter().map(|v| v.edges[edge] == FAR_EDGE).collect();
    if far.iter().any(|f| *f) {
        if !far.iter().all(|f| *f) {
            return Err(format!(
                "edge {edge} is far on corners {far:?} but an edge is far on all three or none"
            ));
        }
        return check_far_shifts(triangle, edge);
    }
    let (b, c) = (points[(edge + 1) % 3], points[(edge + 2) % 3]);
    let altitude = area2 / length(b, c);
    let outward = outward_normal(points, edge);
    for (corner, vertex) in triangle.iter().enumerate() {
        let expected = if corner == edge { altitude } else { 0.0 };
        check_near(
            "distance",
            corner,
            edge,
            vertex.edges[edge],
            expected,
            altitude,
        )?;
        let shift = vertex.normal[0] * outward[0] + vertex.normal[1] * outward[1];
        check_near("shift", corner, edge, vertex.shift[edge], shift, 1.0)?;
        if corner != edge {
            check_moves_out(corner, edge, vertex.normal, shift)?;
        }
    }
    Ok(())
}

/// A corner on an outline edge moves out across it (its miter offset
/// projects to 1 on the edge, or less where clamped): a zero or inward
/// normal leaves the outer half of the coverage ramp undrawn.
fn check_moves_out(corner: usize, edge: usize, normal: [f32; 2], shift: f32) -> Result<(), String> {
    if shift > 0.0 {
        return Ok(());
    }
    Err(format!(
        "corner {corner} on outline edge {edge} has normal {normal:?}, which does not move it out across that edge; write its miter offset"
    ))
}

fn check_far_shifts(triangle: &[MeshVertex; 3], edge: usize) -> Result<(), String> {
    for (corner, vertex) in triangle.iter().enumerate() {
        if vertex.shift[edge] != 0.0 {
            return Err(format!(
                "corner {corner} shift {} on far edge {edge} is not 0; a far edge stays far",
                vertex.shift[edge]
            ));
        }
    }
    Ok(())
}

fn check_near(
    what: &str,
    corner: usize,
    edge: usize,
    found: f32,
    expected: f32,
    scale: f32,
) -> Result<(), String> {
    if (found - expected).abs() <= SLACK * scale.max(1.0) {
        return Ok(());
    }
    Err(format!(
        "corner {corner} {what} {found} to edge {edge} should be {expected} from the positions"
    ))
}

/// The unit normal of edge `edge` pointing away from corner `edge`.
pub(crate) fn outward_normal(points: [[f32; 2]; 3], edge: usize) -> [f32; 2] {
    let (a, b, c) = (points[edge], points[(edge + 1) % 3], points[(edge + 2) % 3]);
    let (dx, dy) = (c[0] - b[0], c[1] - b[1]);
    let len = (dx * dx + dy * dy).sqrt();
    let n = [dy / len, -dx / len];
    let towards_a = (a[0] - b[0]) * n[0] + (a[1] - b[1]) * n[1];
    if towards_a > 0.0 { [-n[0], -n[1]] } else { n }
}

/// Twice the signed area of the triangle over `points`.
pub(crate) fn twice_area([a, b, c]: [[f32; 2]; 3]) -> f32 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

pub(crate) fn length(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

#[cfg(test)]
#[path = "mesh_tests.rs"]
mod tests;

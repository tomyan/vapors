//! A tessellated shape's triangles (docs/plans/one-scene.md 1.2, S3a3),
//! each with its three corners' wedges, and the check that every number
//! in them is one a corner can have: the fragment shader takes coverage
//! from these numbers after the transform, so a wrong one draws a seam
//! or a hard edge rather than failing.
//!
//! The rule, by corner. A corner on the outline carries the outward
//! normals of the two outline edges that bound its wedge (the same one
//! twice where the corner is on a straight run of outline) and whether
//! the wedge is convex (the shape is inside both edges' lines there) or
//! reflex (inside either). A corner the triangles surround is inside:
//! no normals. Coverage is 0.5 plus the least, over a triangle's
//! outline corners, of each wedge's distance in device pixels (convex:
//! the nearer line; reflex: the farther), so it is exact along every
//! edge and round every corner, wherever the triangle lies; the vertex
//! shader moves each outline corner half a device pixel out across both
//! its lines, so the ramp straddles the true outline (Tom, 2026-11-13,
//! option 2; the corner wedges, S3b decided, lead 2026-11-14).

/// A corner's kind: convex, reflex or inside, as the shader reads it.
pub const CONVEX: f32 = 1.0;
/// See [`CONVEX`].
pub const REFLEX: f32 = -1.0;
/// See [`CONVEX`].
pub const INSIDE: f32 = 0.0;

/// One corner of a triangle: 28 bytes, in field order.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshCorner {
    /// Its position in viewBox units.
    pub position: [f32; 2],
    /// The outward unit normals of the two outline edges bounding its
    /// wedge; zero for an inside corner.
    pub normals: [[f32; 2]; 2],
    /// [`CONVEX`], [`REFLEX`] or [`INSIDE`].
    pub kind: f32,
}

impl MeshCorner {
    /// An inside corner at the origin, to fill before writing.
    pub(crate) const INSIDE_AT_ORIGIN: Self = Self {
        position: [0.0; 2],
        normals: [[0.0; 2]; 2],
        kind: INSIDE,
    };
}

/// One triangle, its three corners: 84 bytes, uploaded as they are and
/// drawn as one instance each. The mesh pipeline derives its layout
/// from these fields' `offset_of!` and the struct's size.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshTriangle {
    /// Its corners, in no particular winding.
    pub corners: [MeshCorner; 3],
}

impl MeshTriangle {
    /// Whether the edge opposite corner `edge` is on the outline: both
    /// its ends bound their wedges by its outward normal.
    #[must_use]
    pub fn on_outline(&self, edge: usize) -> bool {
        let normal = outward_normal(self.corners.map(|c| c.position), edge);
        let bounds = |k: usize| {
            self.corners[k]
                .normals
                .iter()
                .any(|m| (m[0] - normal[0]).abs() <= SLACK && (m[1] - normal[1]).abs() <= SLACK)
        };
        bounds((edge + 1) % 3) && bounds((edge + 2) % 3)
    }
}

const _: () = assert!(size_of::<MeshCorner>() == 28);
const _: () = assert!(size_of::<MeshTriangle>() == 84);

/// How far a unit normal's length, or a corner's side of a line, may be
/// off for f32 arithmetic.
const SLACK: f32 = 1e-4;

/// Why `triangles` cannot be drawn, if they cannot: no triangle; a
/// value that is not finite; a triangle without area; a corner whose
/// kind is none of the three, an inside corner with normals, an outline
/// corner whose normals are not unit length, meet head-on, or put its
/// triangle outside its wedge.
///
/// # Errors
/// The reason, naming the triangle and the corner.
pub fn check_mesh(triangles: &[MeshTriangle]) -> Result<(), String> {
    if triangles.is_empty() {
        return Err("a mesh with no triangles draws nothing".to_owned());
    }
    for (t, triangle) in triangles.iter().enumerate() {
        check_triangle(triangle).map_err(|reason| format!("triangle {t} {reason}"))?;
    }
    Ok(())
}

fn check_triangle(triangle: &MeshTriangle) -> Result<(), String> {
    for (c, corner) in triangle.corners.iter().enumerate() {
        check_finite(c, corner)?;
    }
    let points = triangle.corners.map(|corner| corner.position);
    if twice_area(points) == 0.0 {
        return Err(format!("has no area: corners {points:?}"));
    }
    for (c, corner) in triangle.corners.iter().enumerate() {
        check_corner(corner, points).map_err(|reason| format!("corner {c} {reason}"))?;
    }
    Ok(())
}

fn check_finite(c: usize, corner: &MeshCorner) -> Result<(), String> {
    finite(c, "position", &corner.position)?;
    finite(c, "normals", corner.normals.as_flattened())?;
    finite(c, "kind", &[corner.kind])
}

fn finite(c: usize, field: &str, values: &[f32]) -> Result<(), String> {
    if values.iter().all(|v| v.is_finite()) {
        return Ok(());
    }
    Err(format!("corner {c} {field} {values:?} is not finite"))
}

#[allow(clippy::float_cmp)] // exact: the kinds are written as these values
fn check_corner(corner: &MeshCorner, points: [[f32; 2]; 3]) -> Result<(), String> {
    let normals = corner.normals;
    if corner.kind == INSIDE {
        if normals != [[0.0; 2]; 2] {
            return Err(format!(
                "is inside but has normals {normals:?}; an inside corner has none"
            ));
        }
        return Ok(());
    }
    if corner.kind != CONVEX && corner.kind != REFLEX {
        return Err(format!(
            "has kind {}, not convex (1), reflex (-1) or inside (0)",
            corner.kind
        ));
    }
    check_normals(normals)?;
    check_inside_wedge(corner, points)
}

fn check_normals(normals: [[f32; 2]; 2]) -> Result<(), String> {
    for normal in normals {
        let len = length(normal, [0.0, 0.0]);
        if (len - 1.0).abs() > SLACK {
            return Err(format!(
                "has normal {normal:?}, of length {len}, not a unit normal"
            ));
        }
    }
    if head_on(normals) {
        return Err(format!(
            "has normals {normals:?} that meet head-on; its wedge has no inside"
        ));
    }
    Ok(())
}

/// The triangle lies inside the corner's wedge: inside both lines for a
/// convex corner, inside either for a reflex one, at each of its
/// corners.
#[allow(clippy::float_cmp)] // exact: the kinds are written as these values
fn check_inside_wedge(corner: &MeshCorner, points: [[f32; 2]; 3]) -> Result<(), String> {
    let size = points
        .iter()
        .map(|p| length(*p, corner.position))
        .fold(1.0, f32::max);
    for p in points {
        let out = corner
            .normals
            .map(|m| m[0] * (p[0] - corner.position[0]) + m[1] * (p[1] - corner.position[1]));
        let worst = if corner.kind == CONVEX {
            out[0].max(out[1])
        } else {
            out[0].min(out[1])
        };
        if worst > SLACK * size {
            return Err(format!(
                "has a wedge whose normals {:?} put the triangle's corner {p:?} outside it",
                corner.normals
            ));
        }
    }
    Ok(())
}

/// Whether two unit normals meet head-on, as at the end of a cut of no
/// width: the wedge between them has no inside. The builder and
/// [`check_mesh`] both ask this.
pub(crate) fn head_on([n1, n2]: [[f32; 2]; 2]) -> bool {
    n1[0] * n2[0] + n1[1] * n2[1] <= -1.0 + SLACK
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

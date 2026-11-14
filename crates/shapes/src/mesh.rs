//! A tessellated shape's triangles (docs/plans/one-scene.md 1.2, S3a3),
//! each with its three corners' wedges; `mesh_check.rs` checks that
//! every number in them is one a corner can have.
//!
//! The rule, by corner. A corner on the outline carries, for each of
//! the two outline edges that bound its wedge (one edge on each side;
//! the same line twice where the corner is on a straight run), the
//! edge's outward unit normal and its reach: how far the edge runs from
//! the corner, signed, positive along `(-n.y, n.x)`. It also carries
//! whether the wedge is convex (the shape is inside both lines there)
//! or reflex (inside either). A corner the triangles surround is
//! inside: no normals, no reach. Coverage is 0.5 plus the least, over
//! a triangle's outline corners, of each wedge's distance in device
//! pixels (convex: the nearer line, or beyond both lines the distance
//! to the corner; reflex: the farther line), where a line
//! counts only alongside its edge, not past the edge's far end (#371
//! review, H1). The vertex shader moves each outline corner half a
//! device pixel out across both its lines, so the ramp straddles the
//! true outline (Tom, 2026-11-13, option 2; the corner wedges, S3b
//! decided, lead 2026-11-14). A point of a triangle whose nearest
//! outline feature is a corner of another triangle is covered by its
//! own corners' lines only, so near such a corner its coverage is not
//! exact.

/// A corner's kind: convex, reflex or inside, as the shader reads it.
pub const CONVEX: f32 = 1.0;
/// See [`CONVEX`].
pub const REFLEX: f32 = -1.0;
/// See [`CONVEX`].
pub const INSIDE: f32 = 0.0;

/// One corner of a triangle: 36 bytes, in field order.
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
    /// How far each normal's edge runs from the corner, viewBox units,
    /// positive along `(-n.y, n.x)` and negative against it; zero for an
    /// inside corner.
    pub reaches: [f32; 2],
}

impl MeshCorner {
    /// An inside corner at the origin, to fill before writing.
    pub(crate) const INSIDE_AT_ORIGIN: Self = Self {
        position: [0.0; 2],
        normals: [[0.0; 2]; 2],
        kind: INSIDE,
        reaches: [0.0; 2],
    };
}

/// One triangle, its three corners: 108 bytes, uploaded as they are and
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

const _: () = assert!(size_of::<MeshCorner>() == 36);
const _: () = assert!(size_of::<MeshTriangle>() == 108);

/// How far a unit normal's length, or a corner's side of a line, may be
/// off for f32 arithmetic.
pub(crate) const SLACK: f32 = 1e-4;

/// Whether two unit normals meet head-on, as at the end of a cut of no
/// width. The builder and [`crate::check_mesh`] both ask this.
pub(crate) fn head_on([n1, n2]: [[f32; 2]; 2]) -> bool {
    dot(n1, n2) <= -1.0 + SLACK
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

pub(crate) fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

pub(crate) fn minus(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

/// `n` turned a quarter: the direction a positive reach runs.
pub(crate) fn along(n: [f32; 2]) -> [f32; 2] {
    [-n[1], n[0]]
}

#[cfg(test)]
#[path = "mesh_tests.rs"]
mod tests;

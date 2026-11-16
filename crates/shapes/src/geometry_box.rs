//! A geometry's box: in its own units grown by half a stroke, and under
//! an affine transform. One answer for an item's culling bounds, which
//! the elements crate builds, and the painter's check of them.

use crate::geometry::Geometry;
use crate::mesh::MeshTriangle;

impl Geometry<'_> {
    /// Its box in its own units, grown by `by` (half the stroke width
    /// for a stroked closed form; a mesh's triangles hold its stroke).
    #[must_use]
    pub fn local_box(&self, by: f32) -> [[f32; 2]; 2] {
        let [lo, hi] = match *self {
            Geometry::Rect { origin, size, .. } => {
                [origin, [origin[0] + size[0], origin[1] + size[1]]]
            }
            Geometry::Ellipse { centre, radii } => [
                [centre[0] - radii[0], centre[1] - radii[1]],
                [centre[0] + radii[0], centre[1] + radii[1]],
            ],
            Geometry::Mesh { triangles } => mesh_box(triangles),
        };
        [[lo[0] - by, lo[1] - by], [hi[0] + by, hi[1] + by]]
    }
}

/// The box of a mesh's corners: its outline (the extrusion past it
/// draws only the ramp, which a reader adds after the transform).
fn mesh_box(triangles: &[MeshTriangle]) -> [[f32; 2]; 2] {
    let points = triangles.iter().flat_map(|t| t.corners).map(|c| c.position);
    points.fold(
        [[f32::INFINITY; 2], [f32::NEG_INFINITY; 2]],
        |[lo, hi], p| {
            [
                [lo[0].min(p[0]), lo[1].min(p[1])],
                [hi[0].max(p[0]), hi[1].max(p[1])],
            ]
        },
    )
}

/// The box of the four corners of `[lo, hi]` under the affine `m`
/// (`matrix(a, b, c, d, e, f)`), as `[lo, hi]`.
#[must_use]
pub fn transformed_box([lo, hi]: [[f32; 2]; 2], m: [f32; 6]) -> [[f32; 2]; 2] {
    let corners = [
        [lo[0], lo[1]],
        [hi[0], lo[1]],
        [lo[0], hi[1]],
        [hi[0], hi[1]],
    ];
    let points = corners.map(|[x, y]| [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]]);
    let fold = |pick: fn(f32, f32) -> f32, axis: usize| {
        points.iter().map(|p| p[axis]).fold(points[0][axis], pick)
    };
    [
        [fold(f32::min, 0), fold(f32::min, 1)],
        [fold(f32::max, 0), fold(f32::max, 1)],
    ]
}

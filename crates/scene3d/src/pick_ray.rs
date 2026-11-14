//! The ray `pick` casts, and its tests against a box and a triangle.
//! Each is `#[inline]`: called from `pick.rs` without it, they are not
//! inlined into its per-mesh loop, and `pick_cost` measured the
//! every-triangle pick about twice as slow (2026-11-14).

use glam::{Mat4, Vec3};

/// A segment from the near plane to the far plane.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ray {
    pub(crate) from: Vec3,
    pub(crate) to: Vec3,
}

impl Ray {
    /// The segment under `ndc` from depth 0 to 1, through the inverse of
    /// a view-projection.
    #[inline]
    pub(crate) fn through(inverse: Mat4, ndc: [f32; 2]) -> Self {
        let [x, y] = ndc;
        let from = inverse.project_point3(Vec3::new(x, y, 0.0));
        let to = inverse.project_point3(Vec3::new(x, y, 1.0));
        Self { from, to }
    }

    /// The same segment in the space `inverse` maps world space to. An
    /// affine map keeps the parameter from 0 to 1 the world segment's,
    /// so hits in differently scaled meshes compare by it.
    #[inline]
    pub(crate) fn into(self, inverse: Mat4) -> Self {
        let (from, to) = (
            inverse.transform_point3(self.from),
            inverse.transform_point3(self.to),
        );
        Self { from, to }
    }

    /// Whether both ends are finite: a matrix with no finite inverse
    /// gives a ray that is not.
    #[inline]
    pub(crate) fn is_finite(self) -> bool {
        self.from.is_finite() && self.to.is_finite()
    }

    #[inline]
    fn direction(self) -> Vec3 {
        self.to - self.from
    }
}

/// Whether the segment passes through the box `low..high` (slabs). An
/// axis it does not move along must hold its start inside the slab.
#[inline]
pub(crate) fn enters(ray: Ray, (low, high): (Vec3, Vec3)) -> bool {
    let direction = ray.direction();
    let (mut near, mut far) = (0.0_f32, 1.0_f32);
    for axis in 0..3 {
        let (start, step) = (ray.from[axis], direction[axis]);
        if step == 0.0 {
            if start < low[axis] || start > high[axis] {
                return false;
            }
            continue;
        }
        let (a, b) = ((low[axis] - start) / step, (high[axis] - start) / step);
        (near, far) = (near.max(a.min(b)), far.min(a.max(b)));
    }
    near <= far
}

/// Where along the segment it crosses the triangle (Möller-Trumbore),
/// from 0 to 1; either face. Parallel is judged relative to the lengths
/// involved, so it does not depend on the mesh's units.
#[inline]
pub(crate) fn triangle(ray: Ray, [first, second, third]: [Vec3; 3]) -> Option<f32> {
    let direction = ray.direction();
    let (edge1, edge2) = (second - first, third - first);
    let normal1 = direction.cross(edge2);
    let det = edge1.dot(normal1);
    // |det| <= EPSILON * |direction| |edge1| |edge2|, squared: no roots
    let lengths = direction.length_squared() * edge1.length_squared() * edge2.length_squared();
    if det * det <= f32::EPSILON * f32::EPSILON * lengths {
        return None;
    }
    let offset = ray.from - first;
    let u = offset.dot(normal1) / det;
    let normal2 = offset.cross(edge1);
    let v = direction.dot(normal2) / det;
    let along = edge2.dot(normal2) / det;
    let inside = (0.0..=1.0).contains(&u) && v >= 0.0 && u + v <= 1.0;
    (inside && (0.0..=1.0).contains(&along)).then_some(along)
}

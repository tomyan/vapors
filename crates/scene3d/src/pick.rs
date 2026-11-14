//! Which mesh is under a point of the surface (docs/plans/css-for-3d.md
//! C4): a ray from the active camera through the point, tested against
//! each mesh's bounds in its own space, then against the triangles of
//! the meshes whose bounds it enters; the nearest hit wins. It runs per
//! pointer move, not per frame. Cost: one bounds test per mesh, and a
//! triangle test per triangle of each mesh the ray's bounds test lets
//! through (measured by `examples/pick_cost.rs`; the figures are in
//! docs/scene3d.md).

use std::fmt;

use glam::{Mat4, Vec3};

use crate::{GeometryId, Meshes, NodeId, NodeKind, Scene, SceneError, StoreError};

/// Where to pick: a point in surface pixels (+Y down, from the top left)
/// on a surface of `size` pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickAt {
    /// The point.
    pub point: [f32; 2],
    /// The surface's width and height.
    pub size: [f32; 2],
}

/// Why nothing could be picked.
#[derive(Clone, Debug, PartialEq)]
pub enum PickError {
    /// The scene's active camera could not be resolved, or a node read
    /// failed.
    Store(StoreError),
    /// The camera's projection is refused at this surface's aspect.
    Projection(SceneError),
    /// A mesh names geometry its `Meshes` does not hold.
    GeometryNotHeld {
        /// The mesh node.
        node: NodeId,
        /// The geometry it names.
        geometry: GeometryId,
    },
}

impl From<StoreError> for PickError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl fmt::Display for PickError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => write!(f, "nothing can be picked: {error}"),
            Self::Projection(error) => write!(f, "nothing can be picked: {error}"),
            Self::GeometryNotHeld { node, geometry } => write!(
                f,
                "mesh {node} names {geometry}, which its Meshes does not hold: pick with the Meshes the scene's renderer draws from"
            ),
        }
    }
}

impl std::error::Error for PickError {}

/// A segment from the near plane to the far plane.
#[derive(Clone, Copy)]
struct Ray {
    from: Vec3,
    to: Vec3,
}

impl Ray {
    fn through(inverse: Mat4, ndc: [f32; 2]) -> Self {
        let [x, y] = ndc;
        let from = inverse.project_point3(Vec3::new(x, y, 0.0));
        let to = inverse.project_point3(Vec3::new(x, y, 1.0));
        Self { from, to }
    }

    /// The same segment in the space `inverse` maps world space to; its
    /// parameter from 0 to 1 is the world segment's, so hits in different
    /// meshes compare by it.
    fn into(self, inverse: Mat4) -> Self {
        let (from, to) = (
            inverse.transform_point3(self.from),
            inverse.transform_point3(self.to),
        );
        Self { from, to }
    }
}

/// The mesh under `at`, the nearest of those `skip` does not pass over
/// and the scene draws; `None` when the point is over none. Updates the
/// scene's world matrices first.
///
/// # Errors
/// No resolvable active camera, a projection refused at the surface's
/// aspect, or a mesh whose geometry `meshes` does not hold.
pub fn pick(
    scene: &mut Scene,
    meshes: &Meshes,
    at: PickAt,
    skip: impl Fn(NodeId) -> bool,
) -> Result<Option<NodeId>, PickError> {
    scene.update();
    let ray = Ray::through(view_projection(scene, at.size)?.inverse(), ndc(at));
    let mut nearest: Option<(f32, NodeId)> = None;
    for node in nodes(scene)? {
        if skip(node) || !scene.visible(node)? {
            continue;
        }
        if let Some(t) = hit(scene, meshes, node, ray)?
            && nearest.is_none_or(|(best, _)| t < best)
        {
            nearest = Some((t, node));
        }
    }
    Ok(nearest.map(|(_, node)| node))
}

/// Every node under the root, parents first.
fn nodes(scene: &Scene) -> Result<Vec<NodeId>, StoreError> {
    let (mut found, mut stack) = (Vec::new(), vec![scene.root()]);
    while let Some(node) = stack.pop() {
        found.push(node);
        stack.extend(scene.children(node)?.iter().rev());
    }
    Ok(found)
}

fn ndc(at: PickAt) -> [f32; 2] {
    let ([x, y], [width, height]) = (at.point, at.size);
    [2.0 * x / width - 1.0, 1.0 - 2.0 * y / height]
}

/// The active camera's view-projection at the surface's aspect.
fn view_projection(scene: &Scene, [width, height]: [f32; 2]) -> Result<Mat4, PickError> {
    let camera = scene.active_camera()?;
    let NodeKind::Camera { projection, .. } = scene.kind(camera)? else {
        return Err(StoreError::NoCamera.into());
    };
    let matrix = projection
        .matrix(width / height)
        .map_err(PickError::Projection)?;
    Ok(matrix * scene.world(camera)?.inverse())
}

/// Where along `ray` mesh `node` is first hit, if it is a mesh and is.
fn hit(scene: &Scene, meshes: &Meshes, node: NodeId, ray: Ray) -> Result<Option<f32>, PickError> {
    let NodeKind::Mesh { geometry, .. } = scene.kind(node)? else {
        return Ok(None);
    };
    let id = geometry.id();
    let geometry = meshes
        .get(id)
        .ok_or(PickError::GeometryNotHeld { node, geometry: id })?;
    let local = ray.into(scene.world(node)?.inverse());
    let (low, high) = geometry.bounds();
    if !enters(local, low, high) {
        return Ok(None);
    }
    let corner = |i: &u32| geometry.positions()[*i as usize];
    Ok((geometry.indices().as_chunks::<3>().0.iter())
        .filter_map(|corners| triangle(local, corners.each_ref().map(corner)))
        .reduce(f32::min))
}

/// Whether the segment passes through the box `low..high` (slabs).
fn enters(ray: Ray, low: Vec3, high: Vec3) -> bool {
    let direction = ray.to - ray.from;
    let (mut near, mut far) = (0.0_f32, 1.0_f32);
    for axis in 0..3 {
        let (start, step) = (ray.from[axis], direction[axis]);
        if step.abs() < f32::EPSILON {
            if start < low[axis] || start > high[axis] {
                return false;
            }
            continue;
        }
        let (a, b) = ((low[axis] - start) / step, (high[axis] - start) / step);
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    near <= far
}

/// Where along the segment it crosses the triangle (Möller-Trumbore),
/// from 0 to 1; either face.
fn triangle(ray: Ray, [first, second, third]: [Vec3; 3]) -> Option<f32> {
    let direction = ray.to - ray.from;
    let (edge1, edge2) = (second - first, third - first);
    let normal1 = direction.cross(edge2);
    let det = edge1.dot(normal1);
    if det.abs() < f32::EPSILON {
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

#[cfg(test)]
#[path = "pick_tests.rs"]
mod tests;

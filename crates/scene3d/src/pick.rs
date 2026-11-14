//! Which mesh is under a point of the surface (docs/plans/css-for-3d.md
//! C4): a ray from the active camera through the point, tested against
//! each mesh's bounds in its own space, then against the triangles of
//! the meshes whose bounds it enters; the nearest hit wins. It runs per
//! pointer move, not per frame. Cost: one bounds test per mesh, and a
//! triangle test per triangle of each mesh the ray's bounds test lets
//! through (measured by `examples/pick_cost.rs`; the figures are in
//! docs/scene3d.md).

use crate::pick_ray::{Ray, enters, triangle};
use crate::{Meshes, NodeId, NodeKind, PickError, Scene, StoreError};

/// Where to pick: a point in surface pixels (+Y down, from the top left)
/// on a surface of `size` pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickAt {
    /// The point.
    pub point: [f32; 2],
    /// The surface's width and height.
    pub size: [f32; 2],
}

/// The mesh under `at`, the nearest of those `skip` does not pass over
/// and the scene draws; `None` when the point is over none, or outside
/// the surface (`0..=size` on each axis), as a pointer dragged past the
/// surface's edge is. Of meshes hit at exactly the same depth, the first
/// in the renderer's draw order (parents before children, children in
/// order) wins, as its `Less` depth test shows it. Updates the scene's
/// world matrices first.
///
/// # Errors
/// A size not finite and positive, a point not finite, the active
/// camera's view-projection refused ([`Scene::view_projection`]) or with
/// no finite inverse, a visible mesh whose world matrix has no finite
/// inverse, or one whose geometry `meshes` does not hold.
pub fn pick(
    scene: &mut Scene,
    meshes: &Meshes,
    at: PickAt,
    skip: impl Fn(NodeId) -> bool,
) -> Result<Option<NodeId>, PickError> {
    let Some(ndc) = ndc(at)? else {
        return Ok(None);
    };
    scene.update();
    let ray = camera_ray(scene, at.size, ndc)?;
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

/// The point in normalised device coordinates (+Y up), `None` when it
/// is outside the surface.
fn ndc(at: PickAt) -> Result<Option<[f32; 2]>, PickError> {
    let ([x, y], [width, height]) = (at.point, at.size);
    if !(width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0) {
        return Err(PickError::SizeNotPositive(at.size));
    }
    if !(x.is_finite() && y.is_finite()) {
        return Err(PickError::PointNotFinite(at.point));
    }
    let inside = (0.0..=width).contains(&x) && (0.0..=height).contains(&y);
    Ok(inside.then(|| [2.0 * x / width - 1.0, 1.0 - 2.0 * y / height]))
}

/// The world-space segment under `ndc` from the near plane to the far.
fn camera_ray(scene: &Scene, [width, height]: [f32; 2], ndc: [f32; 2]) -> Result<Ray, PickError> {
    let inverse = scene.view_projection(width / height)?.inverse();
    let ray = Ray::through(inverse, ndc);
    if ray.is_finite() {
        return Ok(ray);
    }
    Err(PickError::ViewNotInvertible(scene.active_camera()?))
}

/// Every node under the root, parents first: the renderer's draw order.
fn nodes(scene: &Scene) -> Result<Vec<NodeId>, StoreError> {
    let (mut found, mut stack) = (Vec::new(), vec![scene.root()]);
    while let Some(node) = stack.pop() {
        found.push(node);
        stack.extend(scene.children(node)?.iter().rev());
    }
    Ok(found)
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
    if !local.is_finite() {
        return Err(PickError::MeshNotInvertible(node));
    }
    if !enters(local, geometry.bounds()) {
        return Ok(None);
    }
    let corner = |i: &u32| geometry.positions()[*i as usize];
    Ok((geometry.indices().as_chunks::<3>().0.iter())
        .filter_map(|corners| triangle(local, corners.each_ref().map(corner)))
        .reduce(f32::min))
}

#[cfg(test)]
#[path = "pick_stage.rs"]
mod stage;

#[cfg(test)]
#[path = "pick_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "pick_screen_tests.rs"]
mod screen_tests;

#[cfg(test)]
#[path = "pick_refusal_tests.rs"]
mod refusal_tests;

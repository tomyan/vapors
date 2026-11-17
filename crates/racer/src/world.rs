//! The racer's world in the scene (one-scene 2.2, R3a1): a mesh per part
//! of `world_geometry.rs`, the ground as a plane drawn with
//! `ground_material`, all under a root group that mirrors Y and turns -90
//! degrees about X, so simulation (x, y, z) is the scene's (x, z, y); the
//! car's parts under a group placed by the race; and a camera outside the
//! mirrored group that draws what the Track draws.

use std::collections::BTreeMap;
use std::f32::consts::FRAC_PI_2;

use vapors_scene3d::{GeometryHandle, MeshError, Meshes, Projection, Quat, SceneError};
use vapors_scene3d::{Transform, Vec3};

use crate::model::{Camera, Race};
use crate::world_geometry::{self, Part, Sink};

/// How far the camera stands back from the ground point it looks at,
/// along its own +Z: inside the lens's depth at every zoom.
const CAMERA_DISTANCE: f32 = 1000.0;

/// The ground plane's half side: past what any zoom shows around the car.
const GROUND_HALF: f32 = 2000.0;

/// Each part's triangles: positions and their faces' light as shades.
#[derive(Default)]
struct Pieces {
    parts: BTreeMap<Part, (Vec<Vec3>, Vec<f32>)>,
}

impl Sink for Pieces {
    fn triangle(&mut self, part: Part, points: [[f32; 3]; 3], light: f32) {
        let (positions, shades) = self.parts.entry(part).or_default();
        positions.extend(points.map(Vec3::from));
        shades.extend([light; 3]);
    }
}

/// Every part's geometry, given to `meshes`, in [`Part::ALL`]'s order;
/// a part with no triangles is refused, as [`Meshes::geometry`] refuses
/// empty buffers.
///
/// # Errors
/// As [`Meshes::geometry`].
///
/// # Panics
/// Never: a part has some thousands of corners.
pub fn part_geometry(meshes: &mut Meshes) -> Result<Vec<(Part, GeometryHandle)>, MeshError> {
    let mut pieces = Pieces::default();
    world_geometry::scenery(&mut pieces);
    world_geometry::car(&mut pieces);
    let mut handles = Vec::new();
    for part in Part::ALL {
        let (positions, shades) = pieces.parts.remove(&part).unwrap_or_default();
        let count = u32::try_from(positions.len()).expect("fewer than 2^32 corners");
        handles.push((
            part,
            meshes.geometry(positions, (0..count).collect(), shades)?,
        ));
    }
    Ok(handles)
}

/// The ground: a plane at height 0, `GROUND_HALF` each way from the
/// circuit's centre, in the simulation's X, Y.
///
/// # Errors
/// As [`Meshes::geometry`].
pub fn ground_geometry(meshes: &mut Meshes) -> Result<GeometryHandle, MeshError> {
    let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    let positions = corners
        .map(|(x, y)| Vec3::new(x * GROUND_HALF, y * GROUND_HALF, 0.0))
        .to_vec();
    meshes.geometry(positions, vec![0, 1, 2, 0, 2, 3], vec![1.0; 4])
}

/// The root group: Y mirrored, then turned -90 degrees about X. The
/// Track draws simulation X right and Y down from above, a reflection of
/// a right-handed frame, which no rotation reproduces (one-scene 2.2).
///
/// # Panics
/// Never: the values are finite and nonzero.
#[must_use]
pub fn racers_world() -> Transform {
    let turned = Quat::from_rotation_x(-FRAC_PI_2);
    Transform::new(Vec3::ZERO, turned, Vec3::new(1.0, -1.0, 1.0)).expect("finite and nonzero")
}

/// The car's group in the racer's world: at the race's position, turned
/// by its heading about the simulation's Z.
///
/// # Errors
/// [`SceneError::NonFinite`] for a position the race never holds.
pub fn car_transform(race: &Race) -> Result<Transform, SceneError> {
    let [x, y] = race.position;
    Transform::new(
        Vec3::new(x, y, 0.0),
        Quat::from_rotation_z(race.heading),
        Vec3::ONE,
    )
}

/// The camera's lens: orthographic, `400 / zoom` units across the
/// target's shorter side, the Track's `min(w, h) / 400 * zoom` pixels a
/// unit on any target, the projection taking the target's shape at each
/// render.
///
/// # Errors
/// [`SceneError`] for a zoom that is not finite and positive.
pub fn lens(camera: &Camera) -> Result<Projection, SceneError> {
    Projection::orthographic_shorter(400.0 / camera.zoom, 1.0, 4.0 * CAMERA_DISTANCE)
}

/// The camera in the scene's root, outside the mirrored group (a
/// camera's view inverts its whole world matrix, which
/// `Scene::view_projection` refuses unless it is rigid): looking at the
/// race's position plus the camera's offset, turned by its yaw and
/// tilted as the Track tilts.
///
/// # Errors
/// [`SceneError`] for a camera that is not finite.
pub fn camera_transform(camera: &Camera, race: &Race) -> Result<Transform, SceneError> {
    let target = [0, 1].map(|axis| race.position[axis] + camera.offset[axis]);
    let tilt = camera.tilt.acos();
    let turned = Quat::from_rotation_y(-camera.rotation) * Quat::from_rotation_x(tilt - FRAC_PI_2);
    let back = turned * Vec3::Z * CAMERA_DISTANCE;
    Transform::new(
        Vec3::new(target[0], 0.0, target[1]) + back,
        turned,
        Vec3::ONE,
    )
}

/// `part`'s geometry among `parts` (`part_geometry`'s), for a page's
/// `<Mesh geometry={..}>`; `None`, which the mesh refuses at mount, when
/// it is missing.
#[must_use]
pub fn piece(parts: &[(Part, GeometryHandle)], part: Part) -> Option<GeometryHandle> {
    parts
        .iter()
        .find(|(each, _)| *each == part)
        .map(|(_, handle)| handle.clone())
}

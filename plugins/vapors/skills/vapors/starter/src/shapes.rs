//! Low-poly solids built in code. Each face has its own corners and one
//! baked shade, so the facets read without lights: faces pointing up are
//! lightest, faces towards the camera (+Z) lighter than those away. The
//! shade multiplies the mesh's CSS `color`.

use std::f32::consts::TAU;

use vapors_scene3d::{GeometryHandle, MeshError, Meshes, Vec3};

/// The shapes the page draws with, made once.
#[derive(Clone)]
pub struct Shapes {
    pub cuboid: GeometryHandle,
    pub wedge: GeometryHandle,
    pub octagon: GeometryHandle,
}

impl Shapes {
    /// # Errors
    /// As [`Meshes::geometry`]; never for these fixed shapes.
    pub fn new(meshes: &mut Meshes) -> Result<Self, MeshError> {
        Ok(Self {
            cuboid: cuboid(meshes)?,
            wedge: wedge(meshes)?,
            octagon: cylinder(meshes, 8)?,
        })
    }
}

/// A cube 1 unit wide centred on the origin; scale it into any box.
///
/// # Errors
/// As [`Meshes::geometry`].
pub fn cuboid(meshes: &mut Meshes) -> Result<GeometryHandle, MeshError> {
    let c = |x: f32, y: f32, z: f32| Vec3::new(x * 0.5, y * 0.5, z * 0.5);
    let faces = vec![
        vec![
            c(-1., -1., 1.),
            c(1., -1., 1.),
            c(1., 1., 1.),
            c(-1., 1., 1.),
        ], // front
        vec![
            c(1., -1., -1.),
            c(-1., -1., -1.),
            c(-1., 1., -1.),
            c(1., 1., -1.),
        ], // back
        vec![
            c(-1., 1., 1.),
            c(1., 1., 1.),
            c(1., 1., -1.),
            c(-1., 1., -1.),
        ], // top
        vec![
            c(-1., -1., -1.),
            c(1., -1., -1.),
            c(1., -1., 1.),
            c(-1., -1., 1.),
        ], // bottom
        vec![
            c(1., -1., 1.),
            c(1., -1., -1.),
            c(1., 1., -1.),
            c(1., 1., 1.),
        ], // right
        vec![
            c(-1., -1., -1.),
            c(-1., -1., 1.),
            c(-1., 1., 1.),
            c(-1., 1., -1.),
        ], // left
    ];
    solid(meshes, &faces)
}

/// A wedge 1 unit wide, deep and tall, centred on the origin: full
/// height at the back (-Z), sloping down to the front edge (+Z).
///
/// # Errors
/// As [`Meshes::geometry`].
pub fn wedge(meshes: &mut Meshes) -> Result<GeometryHandle, MeshError> {
    let c = |x: f32, y: f32, z: f32| Vec3::new(x * 0.5, y * 0.5, z * 0.5);
    let faces = vec![
        vec![
            c(-1., -1., 1.),
            c(1., -1., 1.),
            c(1., 1., -1.),
            c(-1., 1., -1.),
        ], // slope
        vec![
            c(1., -1., -1.),
            c(-1., -1., -1.),
            c(-1., 1., -1.),
            c(1., 1., -1.),
        ], // back
        vec![
            c(-1., -1., -1.),
            c(1., -1., -1.),
            c(1., -1., 1.),
            c(-1., -1., 1.),
        ], // bottom
        vec![c(1., -1., 1.), c(1., -1., -1.), c(1., 1., -1.)], // right
        vec![c(-1., -1., -1.), c(-1., -1., 1.), c(-1., 1., -1.)], // left
    ];
    solid(meshes, &faces)
}

/// A cylinder of `sides` flat sides around the Y axis, 1 unit tall and
/// 1 unit across, centred on the origin. Lay it down with a rotation.
///
/// # Errors
/// As [`Meshes::geometry`]; fewer than 3 sides gives no solid.
pub fn cylinder(meshes: &mut Meshes, sides: u32) -> Result<GeometryHandle, MeshError> {
    let rim = |i: u32, y: f32| {
        let angle = TAU * i as f32 / sides as f32;
        Vec3::new(0.5 * angle.sin(), y, 0.5 * angle.cos())
    };
    let mut faces: Vec<Vec<Vec3>> = (0..sides)
        .map(|i| vec![rim(i, -0.5), rim(i + 1, -0.5), rim(i + 1, 0.5), rim(i, 0.5)])
        .collect();
    faces.push((0..sides).map(|i| rim(i, 0.5)).collect());
    faces.push((0..sides).rev().map(|i| rim(i, -0.5)).collect());
    solid(meshes, &faces)
}

/// Flat faces, each a convex polygon wound counter-clockwise seen from
/// outside, each with its own corners and its shade.
fn solid(meshes: &mut Meshes, faces: &[Vec<Vec3>]) -> Result<GeometryHandle, MeshError> {
    let (mut positions, mut indices, mut shades) = (Vec::new(), Vec::new(), Vec::new());
    for face in faces {
        let first = positions.len() as u32;
        let normal = (face[1] - face[0]).cross(face[2] - face[0]).normalize();
        positions.extend_from_slice(face);
        shades.extend(std::iter::repeat_n(shade(normal), face.len()));
        for i in 1..face.len() as u32 - 1 {
            indices.extend([first, first + i, first + i + 1]);
        }
    }
    meshes.geometry(positions, indices, shades)
}

/// Lightest facing up, darker on the sides, darkest facing away from
/// the camera or down.
fn shade(normal: Vec3) -> f32 {
    (0.6 + 0.4 * normal.y + 0.15 * normal.z - 0.05 * normal.x).clamp(0.35, 1.0)
}

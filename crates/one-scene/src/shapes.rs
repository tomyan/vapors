//! The scene's two pieces of geometry, given to its `Meshes` once.

use vapors_scene3d::{GeometryHandle, MeshError, Meshes, Vec3};

/// A cube 1 unit wide, centred on the origin: 8 corners, 12 triangles.
///
/// # Errors
/// As [`Meshes::geometry`]; never for these fixed corners.
pub fn cube(meshes: &mut Meshes) -> Result<GeometryHandle, MeshError> {
    let corners = (0..8)
        .map(|i| {
            let side = |bit: u32| if i & (1 << bit) == 0 { -0.5 } else { 0.5 };
            Vec3::new(side(0), side(1), side(2))
        })
        .collect();
    // each face's corners, counter-clockwise from outside
    let faces = [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ];
    let indices = faces
        .iter()
        .flat_map(|[a, b, c, d]| [*a, *b, *c, *a, *c, *d])
        .collect();
    meshes.geometry(corners, indices)
}

/// A square 12 units wide in the XZ plane, centred on the origin.
///
/// # Errors
/// As [`Meshes::geometry`]; never for these fixed corners.
pub fn ground(meshes: &mut Meshes) -> Result<GeometryHandle, MeshError> {
    let corners = [(-6.0, -6.0), (6.0, -6.0), (6.0, 6.0), (-6.0, 6.0)];
    let positions = corners.map(|(x, z)| Vec3::new(x, 0.0, z)).to_vec();
    meshes.geometry(positions, vec![0, 2, 1, 0, 3, 2])
}

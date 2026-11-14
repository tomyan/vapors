//! The scene's two pieces of geometry, given to its `Meshes` once. A
//! cube's faces carry their own shades (docs/plans/one-scene.md D4), so
//! one CSS colour draws them in different tones; the ground is flat.

use vapors_scene3d::{GeometryHandle, MeshError, Meshes, Vec3};

/// Each face of the cube: its four corners, counter-clockwise seen
/// from outside, each as three bits (x is bit 2, y bit 1, z bit 0; a
/// clear bit is -0.5, a set one 0.5), and its shade.
const FACES: [([u8; 4], f32); 6] = [
    ([0b001, 0b101, 0b111, 0b011], 1.0),  // front, +Z
    ([0b100, 0b000, 0b010, 0b110], 1.0),  // back, -Z
    ([0b011, 0b111, 0b110, 0b010], 0.65), // top, +Y
    ([0b000, 0b100, 0b101, 0b001], 0.65), // bottom, -Y
    ([0b101, 0b100, 0b110, 0b111], 0.8),  // right, +X
    ([0b000, 0b001, 0b011, 0b010], 0.8),  // left, -X
];

/// A cube 1 unit wide, centred on the origin: 24 corners (4 a face, so
/// each face has its own shade), 12 triangles.
///
/// # Errors
/// As [`Meshes::geometry`]; never for these fixed corners.
pub fn cube(meshes: &mut Meshes) -> Result<GeometryHandle, MeshError> {
    let corner = |bits: u8| {
        let side = |bit: u8| if bits & (1 << bit) == 0 { -0.5 } else { 0.5 };
        Vec3::new(side(2), side(1), side(0))
    };
    let positions = FACES
        .iter()
        .flat_map(|(corners, _)| corners.map(corner))
        .collect();
    let shades = FACES.iter().flat_map(|&(_, shade)| [shade; 4]).collect();
    let indices = (0..6u32)
        .flat_map(|face| [0, 1, 2, 0, 2, 3].map(|i| face * 4 + i))
        .collect();
    meshes.geometry(positions, indices, shades)
}

/// A square 12 units wide in the XZ plane, centred on the origin, at
/// shade 1.
///
/// # Errors
/// As [`Meshes::geometry`]; never for these fixed corners.
pub fn ground(meshes: &mut Meshes) -> Result<GeometryHandle, MeshError> {
    let corners = [(-6.0, -6.0), (6.0, -6.0), (6.0, 6.0), (-6.0, 6.0)];
    let positions = corners.map(|(x, z)| Vec3::new(x, 0.0, z)).to_vec();
    meshes.geometry(positions, vec![0, 2, 1, 0, 3, 2], vec![1.0; 4])
}

//! Where things are drawn: the board's squares are world units, the
//! camera looks straight down -Z at the board, and the projection is
//! orthographic, so the board is a flat 2D picture.

use vapors_scene3d::{GeometryHandle, MeshError, Meshes, Projection, Quat, Transform, Vec3};

use crate::game::{HEIGHT, WIDTH};

/// A node centred on `column` and `row`, `width` by `height` squares;
/// row 0 is drawn at the top. `depth` puts it in front of (higher) or
/// behind (lower) what shares its place.
///
/// # Panics
/// For a non-finite place or size.
#[must_use]
pub fn place(column: f32, row: f32, (width, height): (f32, f32), depth: f32) -> Transform {
    let at = Vec3::new(column, (HEIGHT - 1) as f32 - row, depth);
    Transform::new(at, Quat::IDENTITY, Vec3::new(width, height, 1.0)).expect("a finite place")
}

/// The camera over the board's centre, looking down -Z.
///
/// # Panics
/// Never: a fixed, finite place.
#[must_use]
pub fn overhead() -> Transform {
    let centre = Vec3::new((WIDTH - 1) as f32 / 2.0, (HEIGHT - 1) as f32 / 2.0, 10.0);
    Transform::from_translation(centre).expect("a fixed place")
}

/// An orthographic lens a little taller than the board.
///
/// # Errors
/// Never for these fixed numbers.
pub fn lens() -> Result<Projection, String> {
    Projection::orthographic(HEIGHT as f32 + 1.0, 0.1, 20.0).map_err(|e| e.to_string())
}

/// A square one unit wide in the XY plane, facing the camera.
///
/// # Errors
/// As [`Meshes::geometry`]; never for these fixed corners.
pub fn square(meshes: &mut Meshes) -> Result<GeometryHandle, MeshError> {
    let corners = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
    let positions = corners.map(|(x, y)| Vec3::new(x, y, 0.0)).to_vec();
    meshes.geometry(positions, vec![0, 1, 2, 0, 2, 3], vec![1.0; 4])
}

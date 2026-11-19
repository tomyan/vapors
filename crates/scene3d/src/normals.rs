//! Per-vertex normals: their checks, and flat-shaded geometry built
//! from positions and indices (docs/plans/lighting.md, LT1).

use crate::{GeometryHandle, GeometryId, MeshError, Meshes, NormalError};
use glam::Vec3;

/// A normal as uploaded: three floats, in a vertex buffer of their own
/// beside the position and shade buffer, only for geometry that has them.
pub const NORMAL_BYTES: usize = std::mem::size_of::<Vec3>();

/// How far a normal's length may be from 1.
const UNIT_TOLERANCE: f32 = 1e-3;

/// Refuses `normals` that are not one per position, finite and unit.
pub(crate) fn check(
    geometry: GeometryId,
    normals: &[Vec3],
    positions: usize,
) -> Result<(), NormalError> {
    if normals.len() != positions {
        let normals = normals.len();
        return Err(NormalError::Count {
            geometry,
            normals,
            positions,
        });
    }
    if let Some(vertex) = normals.iter().position(|n| !n.is_finite()) {
        return Err(NormalError::NotFinite { geometry, vertex });
    }
    let off = |n: &Vec3| (n.length() - 1.0).abs() > UNIT_TOLERANCE;
    match normals.iter().position(off) {
        Some(vertex) => {
            let length = normals[vertex].length();
            Err(NormalError::NotUnit {
                geometry,
                vertex,
                length,
            })
        }
        None => Ok(()),
    }
}

/// Geometry in which each triangle has its own three vertices and its
/// face's normal, so its edges show as facets.
#[derive(Debug, Clone, PartialEq)]
pub struct FlatShaded {
    /// Three positions a triangle, in the order of the input's indices.
    pub positions: Vec<Vec3>,
    /// `0..positions.len()`.
    pub indices: Vec<u32>,
    /// Each corner's shade, copied from the input vertex it came from.
    pub shades: Vec<f32>,
    /// The face normal at each corner: the cross product of the
    /// triangle's edges, so outward for counter-clockwise triangles.
    pub normals: Vec<Vec3>,
}

/// Splits `positions` and `indices` into flat-shaded triangles.
///
/// # Errors
///
/// [`MeshError`] for the checks [`Meshes::geometry`] makes of positions,
/// indices and shades, and [`NormalError::Degenerate`] for a triangle
/// with no area.
///
/// # Panics
///
/// Never in practice: the flat vertex count is the index count, which
/// the buffer-size check keeps far below `u32::MAX`.
pub fn flat_shaded(
    positions: &[Vec3],
    indices: &[u32],
    shades: &[f32],
) -> Result<FlatShaded, MeshError> {
    crate::mesh_error::check(positions, indices)?;
    crate::mesh_error::check_shades(shades, positions.len())?;
    let mut flat = FlatShaded {
        positions: Vec::new(),
        indices: Vec::new(),
        shades: Vec::new(),
        normals: Vec::new(),
    };
    for (triangle, corners) in indices.as_chunks::<3>().0.iter().enumerate() {
        let at: Vec<usize> = corners.iter().map(|&i| i as usize).collect();
        let normal = face_normal(
            triangle,
            [positions[at[0]], positions[at[1]], positions[at[2]]],
        )?;
        for i in at {
            let next = u32::try_from(flat.positions.len()).expect("sizes checked");
            flat.indices.push(next);
            flat.positions.push(positions[i]);
            flat.shades.push(shades[i]);
            flat.normals.push(normal);
        }
    }
    Ok(flat)
}

/// The unit normal of triangle number `triangle` with `corners`.
fn face_normal(triangle: usize, [a, b, c]: [Vec3; 3]) -> Result<Vec3, NormalError> {
    (b - a)
        .cross(c - a)
        .try_normalize()
        .ok_or(NormalError::Degenerate { triangle })
}

impl Meshes {
    /// As [`Meshes::geometry`], with a unit normal for each position:
    /// the explicit route to geometry a lit mesh can use. A geometry
    /// with normals draws unlit exactly as one without.
    ///
    /// # Errors
    ///
    /// [`MeshError`] as [`Meshes::geometry`], and [`MeshError::Normals`]
    /// for normals not one per position, not finite, or not unit length
    /// within 1e-3, naming the geometry and the vertex.
    pub fn lit_geometry(
        &mut self,
        positions: Vec<Vec3>,
        indices: Vec<u32>,
        shades: Vec<f32>,
        normals: Vec<Vec3>,
    ) -> Result<GeometryHandle, MeshError> {
        self.add(positions, indices, shades, Some(normals))
    }

    /// Geometry from [`flat_shaded`] of the triangles given.
    ///
    /// # Errors
    ///
    /// As [`flat_shaded`].
    pub fn flat_geometry(
        &mut self,
        positions: &[Vec3],
        indices: &[u32],
        shades: &[f32],
    ) -> Result<GeometryHandle, MeshError> {
        let flat = flat_shaded(positions, indices, shades)?;
        self.lit_geometry(flat.positions, flat.indices, flat.shades, flat.normals)
    }
}

#[cfg(test)]
#[path = "normals_tests.rs"]
mod tests;

//! What the normals of a geometry are refused for, each naming the
//! geometry and the vertex, and the fix.

use crate::GeometryId;
use std::fmt;

/// Normals the scene cannot light by.
#[derive(Debug, Clone, PartialEq)]
pub enum NormalError {
    /// Not one normal per position.
    Count {
        /// The geometry refused.
        geometry: GeometryId,
        /// How many normals there are.
        normals: usize,
        /// How many positions there are.
        positions: usize,
    },
    /// A normal with a NaN or infinite component.
    NotFinite {
        /// The geometry refused.
        geometry: GeometryId,
        /// The vertex.
        vertex: usize,
    },
    /// A normal whose length is not 1 within 1e-3.
    NotUnit {
        /// The geometry refused.
        geometry: GeometryId,
        /// The vertex.
        vertex: usize,
        /// Its length.
        length: f32,
    },
    /// A triangle with no area, which has no face normal.
    Degenerate {
        /// The triangle, counting from 0 in the indices.
        triangle: usize,
    },
}

impl fmt::Display for NormalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Count {
                geometry,
                normals,
                positions,
            } => write!(
                f,
                "{geometry} has {normals} normals for {positions} positions: give one unit normal per position"
            ),
            Self::NotFinite { geometry, vertex } => write!(
                f,
                "{geometry}: the normal of vertex {vertex} is NaN or infinite: give finite unit normals"
            ),
            Self::NotUnit {
                geometry,
                vertex,
                length,
            } => write!(
                f,
                "{geometry}: the normal of vertex {vertex} has length {length}, not 1 within 0.001: normalise it, or build flat-shaded geometry with `flat_shaded`"
            ),
            Self::Degenerate { triangle } => write!(
                f,
                "triangle {triangle} has collinear or repeated corners, so no face normal: remove it from the indices before flat shading"
            ),
        }
    }
}

impl std::error::Error for NormalError {}

impl From<NormalError> for crate::MeshError {
    fn from(error: NormalError) -> Self {
        Self::Normals(error)
    }
}

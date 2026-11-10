//! What [`crate::Meshes::geometry`] refuses, each naming the fix.

use glam::Vec3;
use std::fmt;

/// Geometry the scene cannot draw.
#[derive(Debug, Clone, PartialEq)]
pub enum MeshError {
    /// No positions.
    NoPositions,
    /// No indices.
    NoIndices,
    /// An index count that is not a multiple of 3.
    NotTriangles {
        /// The count.
        count: usize,
    },
    /// An index naming no position.
    IndexOutOfRange {
        /// Where it is in the indices.
        at: usize,
        /// Its value.
        index: u32,
        /// How many positions there are.
        positions: usize,
    },
    /// A position that is NaN or infinite.
    NonFinite {
        /// Where it is in the positions.
        at: usize,
    },
}

/// Refuses geometry the scene cannot draw.
pub(crate) fn check(positions: &[Vec3], indices: &[u32]) -> Result<(), MeshError> {
    if positions.is_empty() {
        return Err(MeshError::NoPositions);
    }
    if let Some(at) = positions.iter().position(|p| !p.is_finite()) {
        return Err(MeshError::NonFinite { at });
    }
    if indices.is_empty() {
        return Err(MeshError::NoIndices);
    }
    if !indices.len().is_multiple_of(3) {
        return Err(MeshError::NotTriangles {
            count: indices.len(),
        });
    }
    let outside = |index: &u32| usize::try_from(*index).map_or(true, |i| i >= positions.len());
    match indices.iter().position(outside) {
        Some(at) => Err(MeshError::IndexOutOfRange {
            at,
            index: indices[at],
            positions: positions.len(),
        }),
        None => Ok(()),
    }
}

impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPositions => write!(f, "the geometry has no positions: give at least 3"),
            Self::NoIndices => write!(f, "the geometry has no indices: give 3 per triangle"),
            Self::NotTriangles { count } => write!(
                f,
                "the geometry has {count} indices, not a multiple of 3: give 3 per triangle"
            ),
            Self::IndexOutOfRange {
                at,
                index,
                positions,
            } => write!(
                f,
                "index {at} is {index}, past the {positions} positions: give indices below {positions}"
            ),
            Self::NonFinite { at } => {
                write!(f, "position {at} is NaN or infinite: give finite positions")
            }
        }
    }
}

impl std::error::Error for MeshError {}

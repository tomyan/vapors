//! What [`crate::Meshes::geometry`] refuses, each naming the fix.

use glam::Vec3;
use std::fmt;

/// Geometry the scene cannot draw.
#[derive(Debug, Clone, PartialEq)]
pub enum MeshError {
    /// No positions.
    NoPositions,
    /// Fewer than 3 positions, too few for one triangle.
    TooFewPositions {
        /// The count.
        count: usize,
    },
    /// A buffer past WebGPU's default `maxBufferSize`.
    BufferTooLarge {
        /// `position` or `index`.
        buffer: &'static str,
        /// The buffer's size in bytes.
        bytes: usize,
        /// The limit in bytes.
        limit: usize,
    },
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
    /// Not one shade per position.
    ShadeCount {
        /// How many shades there are.
        shades: usize,
        /// How many positions there are.
        positions: usize,
    },
    /// A shade outside 0 to 1, or not a number.
    ShadeOutOfRange {
        /// Where it is in the shades.
        at: usize,
        /// Its value.
        shade: f32,
    },
}

/// WebGPU's default `maxBufferSize` (256 MiB), the limit every
/// adapter supports, so geometry under it uploads on any of them.
pub(crate) const MAX_BUFFER_BYTES: usize = 256 * 1024 * 1024;

/// A vertex as uploaded: its position's three floats, then its shade.
/// The one answer to the vertex's size: the size check here, the
/// renderer's upload and its pipeline's stride all use it, and
/// `vapors-scene3d-gpu`'s `geometry_tests.rs` and `pipeline_tests.rs`
/// fail when the upload or the attributes disagree with it.
pub const VERTEX_BYTES: usize = std::mem::size_of::<Vec3>() + std::mem::size_of::<f32>();
const INDEX_BYTES: usize = std::mem::size_of::<u32>();

/// Refuses buffers of `positions` and `indices` that exceed
/// [`MAX_BUFFER_BYTES`].
pub(crate) fn check_sizes(positions: usize, indices: usize) -> Result<(), MeshError> {
    let sized = [
        ("vertex", positions, VERTEX_BYTES),
        ("index", indices, INDEX_BYTES),
    ];
    for (buffer, count, each) in sized {
        let bytes = count.saturating_mul(each);
        if bytes > MAX_BUFFER_BYTES {
            let limit = MAX_BUFFER_BYTES;
            return Err(MeshError::BufferTooLarge {
                buffer,
                bytes,
                limit,
            });
        }
    }
    Ok(())
}

/// Refuses shades that are not one per position, each from 0 to 1
/// (docs/plans/one-scene.md D4: the node's colour is multiplied by its
/// vertices' shades). There is no default: a geometry gives every shade.
pub(crate) fn check_shades(shades: &[f32], positions: usize) -> Result<(), MeshError> {
    if shades.len() != positions {
        return Err(MeshError::ShadeCount {
            shades: shades.len(),
            positions,
        });
    }
    match shades.iter().position(|shade| !(0.0..=1.0).contains(shade)) {
        Some(at) => Err(MeshError::ShadeOutOfRange {
            at,
            shade: shades[at],
        }),
        None => Ok(()),
    }
}

/// Refuses geometry the scene cannot draw.
pub(crate) fn check(positions: &[Vec3], indices: &[u32]) -> Result<(), MeshError> {
    if positions.is_empty() {
        return Err(MeshError::NoPositions);
    }
    if positions.len() < 3 {
        return Err(MeshError::TooFewPositions {
            count: positions.len(),
        });
    }
    check_sizes(positions.len(), indices.len())?;
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
            Self::TooFewPositions { count } => write!(
                f,
                "the geometry has {count} positions, too few for a triangle: give at least 3"
            ),
            Self::BufferTooLarge {
                buffer,
                bytes,
                limit,
            } => write!(
                f,
                "the {buffer} buffer would be {bytes} bytes, past WebGPU's default maxBufferSize of {limit} bytes (256 MiB): split the mesh into several geometries"
            ),
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
            Self::ShadeCount { shades, positions } => write!(
                f,
                "the geometry has {shades} shades for {positions} positions: give one shade, from 0 to 1, per position"
            ),
            Self::ShadeOutOfRange { at, shade } => write!(
                f,
                "shade {at} is {shade}, outside 0 to 1: give each vertex's shade as a fraction of the node's colour"
            ),
        }
    }
}

impl std::error::Error for MeshError {}

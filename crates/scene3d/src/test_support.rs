//! Shared test values.

use crate::{Colour, Meshes, NodeKind, Vec3};

/// A mesh node's kind: one triangle in grey.
pub(crate) fn mesh() -> NodeKind {
    let geometry = Meshes::new()
        .geometry(vec![Vec3::ZERO, Vec3::X, Vec3::Y], vec![0, 1, 2])
        .expect("a triangle is valid");
    NodeKind::Mesh {
        geometry,
        colour: Colour::srgb(0.5, 0.5, 0.5).expect("grey is valid"),
    }
}

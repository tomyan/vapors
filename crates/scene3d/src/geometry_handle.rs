//! A claim on a piece of geometry (`docs/plans/3d-2.md` row 5): its
//! [`Meshes`](crate::Meshes) holds the piece while any handle to it
//! lives, the application's and the one each mesh node naming it holds
//! (`NodeKind::Mesh`), and releases it at its next sweep
//! ([`Meshes::release_unclaimed`](crate::Meshes::release_unclaimed))
//! once the last is dropped. The handle shares the piece itself, so
//! what reads a node's triangles on the CPU ([`crate::pick`]) needs no
//! `Meshes` (lead decision, C4c: a mistake made impossible rather than
//! refused).

use std::fmt;
use std::rc::Rc;

use crate::{Geometry, GeometryId};

/// A shared, counted claim on one piece of geometry: clone it to share
/// it, drop it to give it up. Not `Copy`: a copy would claim nothing.
#[derive(Clone)]
pub struct GeometryHandle {
    id: GeometryId,
    geometry: Rc<Geometry>,
}

impl GeometryHandle {
    /// A first handle to `geometry`, issued as `id`, sharing what its
    /// `Meshes` keeps.
    pub(crate) fn claim(id: GeometryId, geometry: &Rc<Geometry>) -> Self {
        let geometry = Rc::clone(geometry);
        Self { id, geometry }
    }

    /// The piece's id, which renderers key their buffers by.
    #[must_use]
    pub fn id(&self) -> GeometryId {
        self.id
    }

    /// The piece's triangles.
    #[must_use]
    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }
}

impl PartialEq for GeometryHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for GeometryHandle {}

impl fmt::Debug for GeometryHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // its Meshes holds one reference while it holds the piece
        let holders = Rc::strong_count(&self.geometry);
        write!(f, "GeometryHandle({}, {holders} holders)", self.id)
    }
}

impl fmt::Display for GeometryHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.id.fmt(f)
    }
}

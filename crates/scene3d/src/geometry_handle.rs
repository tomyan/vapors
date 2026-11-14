//! A claim on a piece of geometry (`docs/plans/3d-2.md` row 5): its
//! [`Meshes`](crate::Meshes) holds the piece while any handle to it
//! lives, the application's and the one each mesh node naming it holds
//! (`NodeKind::Mesh`), and releases it at its next sweep
//! ([`Meshes::release_unclaimed`](crate::Meshes::release_unclaimed))
//! once the last is dropped.

use std::fmt;
use std::rc::{Rc, Weak};

use crate::GeometryId;

/// A shared, counted claim on one piece of geometry: clone it to share
/// it, drop it to give it up. Not `Copy`: a copy would claim nothing.
#[derive(Clone)]
pub struct GeometryHandle {
    id: GeometryId,
    claim: Rc<GeometryId>,
}

impl GeometryHandle {
    /// A first handle to `id`, and what its `Meshes` keeps to see when
    /// the last handle is dropped.
    pub(crate) fn claim(id: GeometryId) -> (Self, Weak<GeometryId>) {
        let claim = Rc::new(id);
        let watch = Rc::downgrade(&claim);
        (Self { id, claim }, watch)
    }

    /// The piece's id, which renderers key their buffers by.
    #[must_use]
    pub fn id(&self) -> GeometryId {
        self.id
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
        let claims = Rc::strong_count(&self.claim);
        write!(f, "GeometryHandle({}, {claims} handles)", self.id)
    }
}

impl fmt::Display for GeometryHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.id.fmt(f)
    }
}

//! A shape parsed and tessellated at run time (docs/plans/one-scene.md
//! 1.5, S4a): the same parts as a table's [`crate::ShapeEntry`], owned,
//! so a document's cache can make one and a painter's store can hold
//! it for as long as either needs it.

use crate::canonical::{Canonical, Paint};
use crate::classify::{Classified, classify};
use crate::geometry::Geometry;
use crate::mesh::MeshTriangle;
use crate::path::IconPath;
use crate::tessellate::tessellate;

/// The flattening tolerance a run-time shape is tessellated at, device
/// pixels at the largest scale it is drawn at (S3b decided, lead
/// 2026-11-14: no difference was measured between 0.25 and 0.05).
pub const TOLERANCE_PX: f32 = 0.1;

/// The flattening tolerance of a table's meshes, viewBox units: a
/// table has no scale, so it takes 0.1 device pixels at 2 px per unit.
/// A change changes the geometry, so bump [`crate::TABLE_FORMAT`].
pub const TABLE_TOLERANCE: f32 = 0.05;

/// One run-time shape.
#[derive(Debug, PartialEq)]
pub struct RuntimeShape {
    site: String,
    canonical: Canonical,
    drawn: Drawn,
    tolerance: f32,
}

/// What a run-time shape draws, owned.
#[derive(Debug, PartialEq)]
enum Drawn {
    Rect {
        origin: [f32; 2],
        size: [f32; 2],
        radius: [f32; 2],
    },
    Ellipse {
        centre: [f32; 2],
        radii: [f32; 2],
    },
    Mesh(Vec<MeshTriangle>),
}

impl RuntimeShape {
    /// The closed form `classified` gives, or the mesh `triangles`
    /// flattened within `tolerance` viewBox units, of the shape whose
    /// canonical form is `canonical`.
    pub(crate) fn new(
        site: &str,
        canonical: Canonical,
        classified: Classified,
        mesh: Option<(Vec<MeshTriangle>, f32)>,
    ) -> Self {
        let (drawn, tolerance) = match (classified, mesh) {
            (
                Classified::Rect {
                    origin,
                    size,
                    radius,
                },
                _,
            ) => (
                Drawn::Rect {
                    origin,
                    size,
                    radius,
                },
                0.0,
            ),
            (Classified::Ellipse { centre, radii }, _) => (Drawn::Ellipse { centre, radii }, 0.0),
            (Classified::Path, Some((triangles, tolerance))) => (Drawn::Mesh(triangles), tolerance),
            (Classified::Path, None) => unreachable!("a path is tessellated before it is kept"),
        };
        Self {
            site: site.to_owned(),
            canonical,
            drawn,
            tolerance,
        }
    }

    /// The shape of `path` painted by `paint` as a crate's table holds
    /// it (S5a): classified, and a mesh at [`TABLE_TOLERANCE`].
    ///
    /// # Errors
    /// A stroke width [`Canonical::of`] refuses, or a path the
    /// tessellator refuses, naming `site`.
    pub fn for_table(site: &str, path: &IconPath, paint: Paint) -> Result<Self, String> {
        let canonical = Canonical::of(site, path, paint)?;
        let classified = classify(path, paint);
        let mesh = if classified == Classified::Path {
            let mesh = tessellate(site, path, paint, TABLE_TOLERANCE)?;
            Some((mesh.triangles, mesh.tolerance))
        } else {
            None
        };
        Ok(Self::new(site, canonical, classified, mesh))
    }

    /// Where it was written, for messages.
    #[must_use]
    pub fn site(&self) -> &str {
        &self.site
    }

    /// Its canonical form.
    #[must_use]
    pub fn canonical(&self) -> &Canonical {
        &self.canonical
    }

    /// What it draws.
    #[must_use]
    pub fn geometry(&self) -> Geometry<'_> {
        match &self.drawn {
            Drawn::Rect {
                origin,
                size,
                radius,
            } => Geometry::Rect {
                origin: *origin,
                size: *size,
                radius: *radius,
            },
            Drawn::Ellipse { centre, radii } => Geometry::Ellipse {
                centre: *centre,
                radii: *radii,
            },
            Drawn::Mesh(triangles) => Geometry::Mesh { triangles },
        }
    }

    /// The flattening tolerance of its mesh, viewBox units; 0 for a
    /// closed form, which is exact.
    #[must_use]
    pub fn tolerance(&self) -> f32 {
        self.tolerance
    }
}

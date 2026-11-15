//! A shape's geometry in viewBox units, and whether it can be drawn.
//! The compiler classifies each literal shape to the cheapest exact
//! primitive (docs/plans/one-scene.md 1.2); the table check refuses a
//! geometry no shader could draw rather than letting it reach one.

use crate::mesh::MeshTriangle;
use crate::mesh_check::check_mesh;

/// A shape's geometry, in viewBox units: the cheapest exact primitive
/// the compiler classified it to (one-scene.md 1.2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Geometry<'a> {
    /// An axis-aligned rectangle, rounded when `radius` is not zero,
    /// drawn by the closed-form pipeline's distance
    /// (`crates/renderer/src/closed_forms.wgsl`).
    Rect {
        /// Its top-left corner.
        origin: [f32; 2],
        /// Width and height, both above zero.
        size: [f32; 2],
        /// Corner radii `rx`, `ry`: zero or more, at most half the
        /// width and the height (SVG clamps them there; the emitter
        /// clamps before the table), both zero or both above it (SVG
        /// draws square corners when either is zero; the emitter writes
        /// both zero).
        radius: [f32; 2],
    },
    /// An ellipse; a circle when the radii are equal.
    Ellipse {
        /// Its centre.
        centre: [f32; 2],
        /// Its radii, both above zero.
        radii: [f32; 2],
    },
    /// Triangles from the tessellator, for every other shape, each
    /// with its own three corners (no index buffer: a corner's wedge
    /// belongs to one triangle; `mesh.rs` says the rule).
    /// Winding is not checked, so the mesh pipeline must not cull
    /// faces.
    Mesh {
        /// The triangles.
        triangles: &'a [MeshTriangle],
    },
}

impl Geometry<'_> {
    /// Whether it is a closed form (a rect or an ellipse), drawn by its
    /// distance, rather than a mesh: the one answer a shape item's kind
    /// is built from.
    #[must_use]
    pub fn is_closed_form(&self) -> bool {
        matches!(self, Self::Rect { .. } | Self::Ellipse { .. })
    }
}

/// Why `geometry` cannot be drawn, if it cannot: a value that is not a
/// finite number, a size or radius that is not above zero, a corner
/// radius that is negative, over half the rect's side, or zero while the
/// other is not, or a mesh [`check_mesh`] refuses.
///
/// # Errors
/// The reason, without the crate and site [`ShapeTable::check`] adds.
pub fn check_geometry(geometry: &Geometry<'_>) -> Result<(), String> {
    match geometry {
        Geometry::Rect {
            origin,
            size,
            radius,
        } => {
            finite("origin", *origin)?;
            positive("size", *size)?;
            check_corners(*size, *radius)
        }
        Geometry::Ellipse { centre, radii } => {
            finite("centre", *centre)?;
            positive("radii", *radii)
        }
        Geometry::Mesh { triangles } => check_mesh(triangles),
    }
}

/// The corner radii the closed-form shader draws: finite, not negative,
/// within half the side (`rect_distance` clamps nothing), and both zero
/// or both above zero: an elliptical corner with one radius zero has
/// no curve for the shader's ellipse distance to follow.
fn check_corners(size: [f32; 2], radius: [f32; 2]) -> Result<(), String> {
    finite("radius", radius)?;
    if radius.iter().any(|r| *r < 0.0) {
        return Err(format!("a corner radius of {radius:?} is negative"));
    }
    if radius[0] > size[0] / 2.0 || radius[1] > size[1] / 2.0 {
        return Err(format!(
            "a corner radius of {radius:?} is over half the size {size:?}; the emitter clamps rx to width / 2 and ry to height / 2, as SVG does"
        ));
    }
    if (radius[0] > 0.0) != (radius[1] > 0.0) {
        return Err(format!(
            "corner radii {radius:?} have one zero; SVG draws square corners when either is zero, so the emitter writes both zero"
        ));
    }
    Ok(())
}

fn finite(name: &str, values: [f32; 2]) -> Result<(), String> {
    if values.iter().all(|v| v.is_finite()) {
        return Ok(());
    }
    Err(format!("{name} {values:?} is not a finite number"))
}

fn positive(name: &str, values: [f32; 2]) -> Result<(), String> {
    finite(name, values)?;
    if values.iter().all(|v| *v > 0.0) {
        return Ok(());
    }
    Err(format!("{name} {values:?} is not above zero"))
}

#[cfg(test)]
#[path = "geometry_tests.rs"]
mod tests;

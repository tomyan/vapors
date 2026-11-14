//! A shape's geometry in viewBox units, and whether it can be drawn.
//! The compiler classifies each literal shape to the cheapest exact
//! primitive (docs/plans/one-scene.md 1.2); the table check refuses a
//! geometry no shader could draw rather than letting it reach one.

use crate::mesh::{MeshVertex, check_mesh};

/// A shape's geometry, in viewBox units: the cheapest exact primitive
/// the compiler classified it to (one-scene.md 1.2).
#[derive(Debug, PartialEq)]
pub enum Geometry {
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
        /// clamps before the table), and equal until the shader draws
        /// elliptical corners (S4b, with `<rect rx ry>`).
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
    /// with its own three vertices (no index buffer: a vertex's edge
    /// distances belong to one triangle; `mesh.rs` says the rule).
    /// Winding is not checked, so the mesh pipeline must not cull
    /// faces.
    Mesh {
        /// The triangles.
        triangles: &'static [[MeshVertex; 3]],
    },
}

/// Why `geometry` cannot be drawn, if it cannot: a value that is not a
/// finite number, a size or radius that is not above zero, a corner
/// radius that is negative, over half the rect's side, or not equal to
/// the other, or a mesh [`check_mesh`] refuses.
///
/// # Errors
/// The reason, without the crate and site [`ShapeTable::check`] adds.
pub fn check_geometry(geometry: &Geometry) -> Result<(), String> {
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
/// within half the side (`rect_distance` clamps nothing), and circular:
/// the emitter writes equal radii, so the comparison is exact, not
/// within a margin.
#[allow(clippy::float_cmp)]
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
    if radius[0] != radius[1] {
        return Err(format!(
            "corner radii {radius:?} differ, and the closed-form shader draws circular corners; elliptical corners come with S4b's <rect rx ry>"
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

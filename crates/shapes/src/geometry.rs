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
    /// (`crates/renderer/src/closed_forms_shared.wgsl`).
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
/// other is not, a radius above zero below [`MIN_RADIUS`], or a mesh
/// [`check_mesh`] refuses.
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
            check_corners(*origin, *size, *radius)
        }
        Geometry::Ellipse { centre, radii } => {
            finite("centre", *centre)?;
            positive("radii", *radii)?;
            not_tiny("radii", *radii)
        }
        Geometry::Mesh { triangles } => check_mesh(triangles),
    }
}

/// The corner radii the closed-form shader draws: finite, not negative,
/// within half the side once [`fitted_radii`] clamps them, and both zero
/// or both above zero: an elliptical corner with one radius zero has
/// no curve for the shader's ellipse distance to follow.
fn check_corners(origin: [f32; 2], size: [f32; 2], radius: [f32; 2]) -> Result<(), String> {
    finite("radius", radius)?;
    if radius.iter().any(|r| *r < 0.0) {
        return Err(format!("a corner radius of {radius:?} is negative"));
    }
    fitted_radii(origin, size, radius)?;
    if (radius[0] > 0.0) != (radius[1] > 0.0) {
        return Err(format!(
            "corner radii {radius:?} have one zero; SVG draws square corners when either is zero, so the emitter writes both zero"
        ));
    }
    if radius[0] > 0.0 {
        not_tiny("corner radii", radius)?;
    }
    Ok(())
}

/// `radius` fitted to a rect at `origin` of `size`: each radius at most
/// half its side, and one over half by no more than the classifier's
/// tolerance clamped to half, as SVG clamps. The tolerance is
/// `classify_near`'s, of the rect's largest coordinate (1 at least). It
/// is "the same position" there, so f32 rounding of a pill's corners
/// (#387 review, H1: `y=0.05 height=1 rx=0.5` gives a height of
/// 0.99999994) does not refuse what the classifier read as exact. It is
/// the one answer that the classifier, this check and the renderer read.
///
/// # Errors
/// A radius over half its side by more than the tolerance.
pub fn fitted_radii(
    origin: [f32; 2],
    size: [f32; 2],
    radius: [f32; 2],
) -> Result<[f32; 2], String> {
    let ends = [
        origin[0],
        origin[1],
        origin[0] + size[0],
        origin[1] + size[1],
    ];
    let scale = ends.iter().fold(1.0, |most: f32, v| most.max(v.abs()));
    let mut fitted = radius;
    for (r, side) in fitted.iter_mut().zip(size) {
        if *r > side / 2.0 && !crate::classify_near::near_value(*r, side / 2.0, scale) {
            return Err(format!(
                "a corner radius of {radius:?} is over half the size {size:?}; the emitter clamps rx to width / 2 and ry to height / 2, as SVG does"
            ));
        }
        *r = r.min(side / 2.0);
    }
    Ok(fitted)
}

/// The smallest radius the closed-form shader takes, viewBox units. It
/// squares a radius and divides by one (`quarter_ellipse`): 1e-9
/// squared is 1e-18, twenty orders above f32's smallest normal number
/// (1.2e-38), so a GPU that flushes denormals to zero still gets a
/// number. A smaller radius underflows to 0 there, and coverage is NaN.
pub const MIN_RADIUS: f32 = 1e-9;

/// A refusal when a radius of `values` is below [`MIN_RADIUS`].
fn not_tiny(name: &str, values: [f32; 2]) -> Result<(), String> {
    if values.iter().all(|v| *v >= MIN_RADIUS) {
        return Ok(());
    }
    Err(format!(
        "{name} {values:?} include one below {MIN_RADIUS}, which the shader's arithmetic underflows; draw it as a square corner or a point, or scale the viewBox up"
    ))
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

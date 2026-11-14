//! A mesh item checked for drawing: a finite transform with area, and
//! how far its outline moves out, in its own units, for the half pixel
//! of ramp outside it (Tom, 2026-11-13, option 2; docs/plans/one-scene.md
//! 1.2). Only [`MeshShape::new`] makes one.

use vapors_scene::Color;

use crate::ClosedFormError;
use crate::closed_form_shape::{check_transform, refused};

/// Floats per instance: three four-float attributes.
pub(crate) const INSTANCE_FLOATS: usize = 12;

/// How far the outline moves out, device pixels: the ramp's outer half.
const EXTRUSION: f32 = 0.5;

/// A mesh entry's item: its transform, colour and extrusion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshShape {
    transform: [f32; 6],
    /// Its own units per half device pixel along the direction the
    /// transform shrinks most, so the outline moves at least half a
    /// pixel out whatever way it faces.
    extrusion: f32,
    colour: Color,
}

impl MeshShape {
    /// An entry drawn under `transform` (its units to device pixels, as
    /// CSS's `matrix(a, b, c, d, e, f)`) in `colour`; `None` when the
    /// transform has no area: it draws nothing.
    ///
    /// # Errors
    /// A transform with a value, a determinant or an extrusion that is
    /// not finite, naming it, the fix and a reproducing command.
    pub fn new(transform: [f32; 6], colour: Color) -> Result<Option<Self>, ClosedFormError> {
        let determinant = check_transform(transform, REPRODUCE)?;
        if determinant == 0.0 {
            return Ok(None);
        }
        let extrusion = extrusion(transform)?;
        Ok(Some(Self {
            transform,
            extrusion,
            colour,
        }))
    }

    /// The three four-float attributes `meshes.wgsl` reads per
    /// instance: the transform's linear part; its translation and the
    /// extrusion; the colour.
    pub(crate) fn instance(&self) -> [f32; INSTANCE_FLOATS] {
        let [linear @ .., e, f] = self.transform;
        let c = self.colour;
        [linear, [e, f, self.extrusion, 0.0], [c.r, c.g, c.b, c.a]]
            .concat()
            .try_into()
            .expect("three vec4s")
    }
}

/// The command that reproduces a mesh item's refusal.
const REPRODUCE: &str = "cargo test -p vapors-renderer --lib mesh_shape";

/// Half a device pixel in the transform's units along the direction it
/// shrinks most, worked in f64 so a value past f32's square root does
/// not overflow on the way.
///
/// # Errors
/// An extrusion that is not a finite f32 (singular values past f32's
/// range), naming the transform.
fn extrusion(transform: [f32; 6]) -> Result<f32, ClosedFormError> {
    let [a, b, c, d, ..] = transform.map(f64::from);
    let determinant = a * d - b * c;
    let squares = a * a + b * b + c * c + d * d;
    let spread = (squares * squares - 4.0 * determinant * determinant)
        .max(0.0)
        .sqrt();
    let larger = f64::midpoint(squares, spread).sqrt();
    #[allow(clippy::cast_possible_truncation)] // checked finite just below
    let extrusion = (f64::from(EXTRUSION) * larger / determinant.abs()) as f32;
    if extrusion.is_finite() {
        return Ok(extrusion);
    }
    Err(refused(
        &format!(
            "transform {transform:?} shrinks some direction so far that half a pixel in its units is past f32's range"
        ),
        "give the element a transform whose scale is within f32's range",
        REPRODUCE,
    ))
}

#[cfg(test)]
#[path = "mesh_shape_tests.rs"]
mod tests;

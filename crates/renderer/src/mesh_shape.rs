//! A mesh item checked for drawing: a finite transform with area, and
//! how far its outline moves out, in its own units, for the half pixel
//! of ramp outside it (Tom, 2026-11-13, option 2; docs/plans/one-scene.md
//! 1.2). Only [`MeshShape::new`] makes one.

use vapors_scene::Color;

use crate::ClosedFormError;

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
    /// A transform with a value or a determinant that is not finite,
    /// naming it, the fix and a reproducing command.
    pub fn new(transform: [f32; 6], colour: Color) -> Result<Option<Self>, ClosedFormError> {
        let determinant = crate::closed_form_shape::check_transform(transform)?;
        if determinant == 0.0 {
            return Ok(None);
        }
        let shrinks_most = smaller_singular_value(transform, determinant);
        Ok(Some(Self {
            transform,
            extrusion: EXTRUSION / shrinks_most,
            colour,
        }))
    }

    /// The three four-float attributes `meshes.wgsl` reads per
    /// instance: the transform's linear part; its translation and the
    /// extrusion; the colour.
    pub(crate) fn instance(&self) -> [f32; 12] {
        let [linear @ .., e, f] = self.transform;
        let c = self.colour;
        [linear, [e, f, self.extrusion, 0.0], [c.r, c.g, c.b, c.a]]
            .concat()
            .try_into()
            .expect("three vec4s")
    }
}

/// The smaller singular value of the transform's linear part: the least
/// device length a unit of its own length can become.
fn smaller_singular_value([a, b, c, d, ..]: [f32; 6], determinant: f32) -> f32 {
    let squares = a * a + b * b + c * c + d * d;
    let spread = (squares * squares - 4.0 * determinant * determinant)
        .max(0.0)
        .sqrt();
    let larger = f32::midpoint(squares, spread).sqrt();
    determinant.abs() / larger
}

#[cfg(test)]
#[path = "mesh_shape_tests.rs"]
mod tests;

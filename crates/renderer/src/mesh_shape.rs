//! A mesh item checked for drawing: a finite transform with area and a
//! finite inverse, which the vertex shader needs to move each outline
//! vertex half a device pixel out in device space and carry the move
//! back to the mesh's own units (Tom, 2026-11-13, option 2; the move
//! in device space, lead, 2026-11-14; docs/plans/one-scene.md 1.2).
//! Only [`MeshShape::new`] makes one.

use vapors_scene::Color;

use crate::ClosedFormError;
use crate::closed_form_shape::{check_transform, refused};

/// Floats per instance: three four-float attributes.
pub(crate) const INSTANCE_FLOATS: usize = 12;

/// The command that reproduces a mesh item's refusal.
const REPRODUCE: &str = "cargo test -p vapors-renderer --lib mesh_shape";

/// A mesh entry's item: its transform, the inverse of its determinant,
/// and its colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshShape {
    transform: [f32; 6],
    inverse_determinant: f32,
    colour: Color,
}

impl MeshShape {
    /// An entry drawn under `transform` (its units to device pixels, as
    /// CSS's `matrix(a, b, c, d, e, f)`) in `colour`; `None` when the
    /// transform has no area: it draws nothing.
    ///
    /// # Errors
    /// A transform with a value or determinant that is not finite, or
    /// whose inverse is not (a scale too small for f32), naming it, the
    /// fix and a reproducing command.
    pub fn new(transform: [f32; 6], colour: Color) -> Result<Option<Self>, ClosedFormError> {
        let determinant = check_transform(transform, REPRODUCE)?;
        if determinant == 0.0 {
            return Ok(None);
        }
        let inverse_determinant = check_inverse(transform, determinant)?;
        Ok(Some(Self {
            transform,
            inverse_determinant,
            colour,
        }))
    }

    /// The three four-float attributes `meshes.wgsl` reads per
    /// instance: the transform's linear part; its translation and the
    /// inverse of its determinant; the colour.
    pub(crate) fn instance(&self) -> [f32; INSTANCE_FLOATS] {
        let [linear @ .., e, f] = self.transform;
        let c = self.colour;
        [
            linear,
            [e, f, self.inverse_determinant, 0.0],
            [c.r, c.g, c.b, c.a],
        ]
        .concat()
        .try_into()
        .expect("three vec4s")
    }
}

/// The inverse of `determinant`, when it and every entry of the
/// inverse transform are finite.
fn check_inverse(transform: [f32; 6], determinant: f32) -> Result<f32, ClosedFormError> {
    let inverse = 1.0 / determinant;
    let entries = [transform[0], transform[1], transform[2], transform[3]];
    if inverse.is_finite() && entries.iter().all(|v| (v * inverse).is_finite()) {
        return Ok(inverse);
    }
    Err(refused(
        &format!("transform {transform:?} has no finite inverse: its scale is too small for f32"),
        "give the element a transform whose scale is within f32's range",
        REPRODUCE,
    ))
}

#[cfg(test)]
#[path = "mesh_shape_tests.rs"]
mod tests;

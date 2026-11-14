//! A closed-form shape checked for drawing: its geometry by the shapes
//! crate's own check ([`check_geometry`], the one answer to what can be
//! drawn), its stroke width and its transform here. Only
//! [`ClosedFormShape::new`] makes one, so the pipeline draws nothing
//! unchecked.

use std::fmt;

use vapors_scene::Color;
use vapors_shapes::{Geometry, check_geometry};

/// A rect or ellipse to draw, filled or stroked, under a transform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClosedFormShape {
    /// 0 a rect, 1 an ellipse.
    kind: f32,
    /// A rect's origin and size, or an ellipse's centre and radii.
    numbers: [f32; 4],
    /// A rect's corner radius.
    radius: f32,
    /// The stroke width, when stroked.
    stroke: Option<f32>,
    transform: [f32; 6],
    colour: Color,
}

/// Why a shape cannot be drawn by the closed-form pipeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosedFormError(pub String);

impl ClosedFormShape {
    /// `geometry`, stroked `stroke` wide in its own units (`None`
    /// fills it), under `transform` (its units to device pixels, as
    /// CSS's `matrix(a, b, c, d, e, f)`), in `colour`. `None` when the
    /// transform has no area (CSS's `scale(0)`): it draws nothing.
    ///
    /// # Errors
    /// A geometry [`check_geometry`] refuses, a mesh, a stroke width
    /// that is not a finite number above zero, or a transform with a
    /// value or a determinant that is not finite; the error names the
    /// value, the fix and a reproducing command.
    pub fn new(
        geometry: &Geometry,
        stroke: Option<f32>,
        transform: [f32; 6],
        colour: Color,
    ) -> Result<Option<Self>, ClosedFormError> {
        check_geometry(geometry).map_err(|reason| {
            refused(
                &reason,
                "the shape table's check refuses this geometry, so fix the emitter that wrote it",
                REPRODUCE,
            )
        })?;
        let (kind, numbers, radius) = form(geometry)?;
        check_stroke(stroke)?;
        if check_transform(transform, REPRODUCE)? == 0.0 {
            return Ok(None);
        }
        Ok(Some(Self {
            kind,
            numbers,
            radius,
            stroke,
            transform,
            colour,
        }))
    }

    /// The five four-float attributes `closed_forms.wgsl` reads: the
    /// transform's linear part; its translation, the kind and the
    /// stroke width; the form's four numbers; the radius and whether
    /// it is stroked (1) or filled (0); the colour.
    pub(crate) fn instance(&self) -> [f32; 20] {
        let [linear @ .., e, f] = self.transform;
        let (stroked, width) = self.stroke.map_or((0.0, 0.0), |w| (1.0, w));
        let colour = self.colour;
        [
            linear,
            [e, f, self.kind, width],
            self.numbers,
            [self.radius, stroked, 0.0, 0.0],
            [colour.r, colour.g, colour.b, colour.a],
        ]
        .concat()
        .try_into()
        .expect("five vec4s")
    }
}

/// The kind, the four numbers and the radius of a checked closed form.
fn form(geometry: &Geometry) -> Result<(f32, [f32; 4], f32), ClosedFormError> {
    match *geometry {
        // check_geometry refuses radii that differ.
        Geometry::Rect {
            origin,
            size,
            radius,
        } => Ok((0.0, [origin[0], origin[1], size[0], size[1]], radius[0])),
        Geometry::Ellipse { centre, radii } => {
            Ok((1.0, [centre[0], centre[1], radii[0], radii[1]], 0.0))
        }
        Geometry::Mesh { .. } => Err(refused(
            "a mesh is not a closed form",
            "draw a mesh entry with the mesh pipeline (one-scene S3a2)",
            REPRODUCE,
        )),
    }
}

fn check_stroke(stroke: Option<f32>) -> Result<(), ClosedFormError> {
    match stroke {
        Some(width) if !(width.is_finite() && width > 0.0) => Err(refused(
            &format!("stroke width {width} is not a finite number above zero"),
            "give the shape a stroke-width above zero",
            REPRODUCE,
        )),
        _ => Ok(()),
    }
}

/// `transform`'s determinant, when it and every value are finite; zero
/// for a transform with no area.
pub(crate) fn check_transform(
    transform: [f32; 6],
    reproduce: &str,
) -> Result<f32, ClosedFormError> {
    let fix = "give the element a transform whose values and scale are finite";
    if !transform.iter().all(|v| v.is_finite()) {
        return Err(refused(
            &format!("transform {transform:?} is not finite"),
            fix,
            reproduce,
        ));
    }
    let [a, b, c, d, ..] = transform;
    let determinant = a * d - b * c;
    if !determinant.is_finite() {
        return Err(refused(
            &format!("transform {transform:?} has determinant {determinant}, which is not finite"),
            fix,
            reproduce,
        ));
    }
    Ok(determinant)
}

/// The command that reproduces a closed form's refusal.
const REPRODUCE: &str = "cargo test -p vapors-renderer --lib closed_form_shape";

/// The refusal of `what`, with `fix` and the command, `reproduce`, that
/// runs the tests of that refusal.
pub(crate) fn refused(what: &str, fix: &str, reproduce: &str) -> ClosedFormError {
    ClosedFormError(format!("{what}; {fix}; reproduce with `{reproduce}`"))
}

impl fmt::Display for ClosedFormError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ClosedFormError {}

#[cfg(test)]
#[path = "closed_form_shape_tests.rs"]
mod tests;

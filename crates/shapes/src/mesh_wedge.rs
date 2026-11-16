//! One outline corner's wedge from the walk round its vertex
//! (`mesh_normals.rs`): its two bounding edges' normals and reaches,
//! and whether it is convex or reflex. The kind has two answers: the
//! angle the walk's triangles span, and the turn of the outline itself
//! (whether the second edge's far end lies outside the first edge's
//! line) (#371 review, M1). Where they disagree one way the shape is
//! refused; the other way is let through (`check_turn` says why).

use std::f32::consts::PI;

use crate::mesh::{CONVEX, MeshCorner, REFLEX, along, dot, head_on, length, minus};

/// An outline edge bounding a corner's wedge: its outward unit normal
/// and its far end.
pub(crate) struct Bound {
    pub normal: [f32; 2],
    pub far: [f32; 2],
}

/// How far from straight, radians, the walk's angle must be to decide
/// the kind, and the turn's offset, as a share of the edge's length,
/// to contradict it.
const STRAIGHT: f32 = 1e-4;

/// The corner at `position` bounded by `bounds`, whose triangles span
/// `angle` at it.
///
/// # Errors
/// A reflex corner whose normals meet head-on (the end of a cut of no
/// width into the shape); a corner the walk calls convex where the
/// outline turns reflex. Each names the vertex.
pub(crate) fn outline_corner(
    position: [f32; 2],
    bounds: [Bound; 2],
    angle: f32,
) -> Result<MeshCorner, String> {
    let normals = bounds.each_ref().map(|bound| bound.normal);
    let kind = if angle > PI + STRAIGHT {
        REFLEX
    } else {
        CONVEX
    };
    if kind == REFLEX && head_on(normals) {
        return Err(format!(
            "outline edges meet head-on at {position:?}, where a cut of no width into the shape ends, with no outside to its wedge; close the cut or give it width"
        ));
    }
    check_turn(position, &bounds, kind, angle)?;
    Ok(MeshCorner {
        position,
        normals,
        kind,
        reaches: bounds.map(|bound| dot(minus(bound.far, position), along(bound.normal))),
    })
}

/// The outline's own turn at `position` against the walk's `kind`.
///
/// Relaxed guard (lead, 2026-11-14, #371 review M1): the walk saying
/// reflex where the turn says convex is let through as reflex. Lyon's
/// stroker emits triangles that overlap round some inner joins (touche's
/// Open icon at (2.25, 4.75), pinned by `gpu_mesh_insides.rs`), which
/// inflates the angle sum while the outline turns convex. The fan's
/// triangles are the shape, so reflex (the farther line) only covers
/// more inside them; convex would cut a hole where a triangle reaches
/// past the other edge's line. Overlap only adds angle, so it cannot
/// make the walk say convex where the turn says reflex: that case is
/// refused. A builder that resolves overlapping fans would retire the
/// relaxation (one-scene 1.2).
#[allow(clippy::float_cmp)] // exact: the kinds are written as these values
fn check_turn(
    position: [f32; 2],
    bounds: &[Bound; 2],
    kind: f32,
    angle: f32,
) -> Result<(), String> {
    let far = minus(bounds[1].far, position);
    let out = dot(bounds[0].normal, far);
    let decided = STRAIGHT * length(far, [0.0, 0.0]);
    if kind == REFLEX || out <= decided {
        return Ok(());
    }
    Err(format!(
        "the triangles round {position:?} span {} degrees there, under half a turn, but the outline turns reflex; the triangles leave part of the corner uncovered, so report the path as a fixture",
        angle.to_degrees()
    ))
}

#[cfg(test)]
#[path = "mesh_wedge_tests.rs"]
mod tests;

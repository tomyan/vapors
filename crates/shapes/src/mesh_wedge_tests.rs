//! The two answers to a corner's kind: a walk calling a corner convex
//! where the outline turns reflex is refused, naming the vertex; the
//! other disagreement is let through (the relaxed guard in
//! `mesh_wedge.rs`).

use super::*;

/// A corner at the origin bounded by the edge to (4, 0) (outward -y)
/// and the edge to (2, -3), whose far end lies outside the first line:
/// the outline turns reflex.
fn reflex_turn() -> [Bound; 2] {
    let len = 13f32.sqrt();
    [
        Bound {
            normal: [0.0, -1.0],
            far: [4.0, 0.0],
        },
        Bound {
            normal: [-3.0 / len, -2.0 / len],
            far: [2.0, -3.0],
        },
    ]
}

#[test]
fn a_walk_calling_a_reflex_turn_convex_is_refused_naming_the_vertex() {
    // When: the walk spans 60 degrees
    let refused = outline_corner([0.0, 0.0], reflex_turn(), 60f32.to_radians());

    // Then
    let reason = refused.expect_err("refused");
    assert!(
        reason.starts_with("the triangles round [0.0, 0.0] span 60"),
        "{reason}"
    );
}

#[test]
fn a_walk_calling_a_reflex_turn_reflex_builds() {
    // When: the walk spans 300 degrees
    let corner = outline_corner([0.0, 0.0], reflex_turn(), 300f32.to_radians());

    // Then
    assert_eq!(corner.map(|c| c.kind), Ok(REFLEX));
}

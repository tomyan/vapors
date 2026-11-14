//! Each corner's wedge: the outward normals of the outline edges its
//! walk meets on either side, convex or reflex by the angle the walk
//! spans; inside where the triangles surround the vertex; per corner
//! where the outline touches itself; refused head-on.

use crate::mesh::{CONVEX, INSIDE, REFLEX};
use crate::mesh_check::check_mesh;
use crate::mesh_edges::mesh_triangles;
use crate::mesh_edges::tests::built;

/// Whether `a` and `b` agree to f32 rounding.
fn near(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6
}

#[test]
fn the_inner_corner_of_an_l_is_reflex() {
    // Given: an L, its inner corner (1, 1) at vertex 3, fanned from (0, 0)
    let points = [
        [0.0, 0.0],
        [2.0, 0.0],
        [2.0, 1.0],
        [1.0, 1.0],
        [1.0, 2.0],
        [0.0, 2.0],
    ];

    // When
    let triangles = built(&points, &[0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 5]);

    // Then: both triangles at (1, 1) span 135 degrees each; the walk
    // from (0, 2, 3) across the shared edge to (0, 0) meets the side of
    // the upright (outward +x), the other way the top of the foot (+y)
    let corner = triangles[1].corners[2];
    assert_eq!(corner.position, [1.0, 1.0]);
    assert_eq!(corner.normals, [[1.0, 0.0], [0.0, 1.0]]);
    assert_eq!(corner.reaches, [1.0, -1.0]);
    assert_eq!(corner.kind, REFLEX);
    assert_eq!(triangles[2].corners[1].kind, REFLEX);
    assert_eq!(triangles[0].corners[0].kind, CONVEX);
    assert_eq!(check_mesh(&triangles), Ok(()));
}

#[test]
fn a_corner_on_a_straight_run_is_convex_with_one_normal_twice() {
    // Given: (1, 0) on the bottom of a rectangle, fanned from it
    let points = [[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]];

    // When
    let triangles = built(&points, &[1, 2, 3, 1, 3, 4, 1, 4, 0]);

    // Then
    let corner = triangles[1].corners[0];
    assert_eq!(corner.normals, [[0.0, -1.0], [0.0, -1.0]]);
    assert_eq!(corner.kind, CONVEX);
}

#[test]
fn a_vertex_the_triangles_surround_is_inside() {
    // Given: a unit square fanned round its centre, vertex 4
    let points = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.5, 0.5]];

    // When
    let triangles = built(&points, &[4, 0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0]);

    // Then
    assert!(
        triangles
            .iter()
            .all(|t| t.corners[0].kind == INSIDE && t.corners[0].normals == [[0.0; 2]; 2])
    );
    assert_eq!(check_mesh(&triangles), Ok(()));
}

#[test]
fn each_wedge_of_a_bowtie_is_bounded_by_its_own_side() {
    // Given: two triangles touching at the origin, one opening to +x and
    // one to -x; the vertex has four outline edges
    let points = [
        [0.0, 0.0],
        [2.0, -1.0],
        [2.0, 1.0],
        [-2.0, 1.0],
        [-2.0, -1.0],
    ];

    // When
    let triangles = built(&points, &[0, 1, 2, 0, 3, 4]);

    // Then: the corner opening to +x is bounded by its edges' normals
    // (-1, -2) / sqrt 5 and (-1, 2) / sqrt 5, the other by their mirror
    let s = 5f32.sqrt();
    let [right, left] = [triangles[0].corners[0], triangles[1].corners[0]];
    assert!(near(right.normals[0], [-1.0 / s, -2.0 / s]), "{right:?}");
    assert!(near(right.normals[1], [-1.0 / s, 2.0 / s]), "{right:?}");
    assert!(near(left.normals[0], [1.0 / s, 2.0 / s]), "{left:?}");
    assert!(near(left.normals[1], [1.0 / s, -2.0 / s]), "{left:?}");
    assert_eq!(check_mesh(&triangles), Ok(()));
}

#[test]
fn outline_edges_meeting_head_on_are_refused_naming_the_vertex() {
    // Given: a fan right round the origin from (4, 0) to (2, 0), a slit
    // along +x; the corner's wedge is bounded by the slit's two sides,
    // whose outward normals are (0, -1) and (0, 1)
    let points = [
        [0.0, 0.0],
        [4.0, 0.0],
        [0.0, 2.0],
        [-2.0, 0.0],
        [0.0, -2.0],
        [2.0, 0.0],
    ];

    // When
    let refused = mesh_triangles(&points, &[0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 5]);

    // Then
    assert_eq!(
        refused,
        Err("outline edges meet head-on at [0.0, 0.0], where a cut of no width into the shape ends, with no outside to its wedge; close the cut or give it width".to_owned())
    );
}

#[test]
fn a_convex_needle_builds() {
    // Given: a spike of 0.57 degrees at the origin, whose edges' normals
    // are within 1e-4 of opposite
    let points = [[0.0, 0.0], [100.0, 0.5], [100.0, -0.5]];

    // When
    let triangles = built(&points, &[0, 1, 2]);

    // Then
    assert_eq!(triangles[0].corners[0].kind, CONVEX);
    assert_eq!(check_mesh(&triangles), Ok(()));
}

#[test]
fn triangles_overlapping_round_a_vertex_keep_the_walks_reflex() {
    // Given: a fan at the origin from (4, 0) up to (1, 1), back down to
    // (4, -0.5) and round to (0, 4); its angles sum past half a turn,
    // though the outline from (4, 0) to (0, 4) turns convex, as lyon's
    // stroker leaves some inner joins
    let points = [[0.0, 0.0], [4.0, 0.0], [1.0, 1.0], [4.0, -0.5], [0.0, 4.0]];

    // When
    let triangles = built(&points, &[0, 1, 2, 0, 2, 3, 0, 3, 4]);

    // Then: the relaxed guard in mesh_wedge.rs keeps it reflex
    assert!(triangles.iter().all(|t| t.corners[0].kind == REFLEX));
}

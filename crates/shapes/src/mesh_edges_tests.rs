//! From a shared-vertex mesh: outline edges by use count, altitudes,
//! interior edges far, normals as miter offsets with their shifts,
//! flat triangles dropped before counting.

use super::*;
use crate::mesh::check_mesh;

/// A unit square as two triangles sharing the diagonal 0-2.
const SQUARE: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
const SQUARE_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];

#[test]
fn a_squares_outer_edges_are_outline_and_its_diagonal_is_far() {
    // When
    let triangles = triangles_with_edges(&SQUARE, &SQUARE_INDICES);

    // Then: the first triangle (0, 1, 2); edge 0 is 1-2 (outline), edge
    // 1 is 2-0 (the diagonal, shared), edge 2 is 0-1 (outline)
    assert_eq!(triangles.len(), 2);
    let [a, b, c] = triangles[0];
    assert_eq!(a.position, [0.0, 0.0]);
    assert_eq!(a.edges, [1.0, FAR_EDGE, 0.0]);
    assert_eq!(b.edges, [0.0, FAR_EDGE, 0.0]);
    assert_eq!(c.edges, [0.0, FAR_EDGE, 1.0]);
    assert_eq!(check_mesh(&triangles), Ok(()));
}

#[test]
fn a_corner_moves_along_the_miter_of_its_outline_edges_with_shift_one_on_each() {
    // When
    let triangles = triangles_with_edges(&SQUARE, &SQUARE_INDICES);

    // Then: corner (0, 0) sits between the bottom edge (outward -y)
    // and the left edge (outward -x), so it moves (-1, -1); on its own
    // outline edge 2 (bottom) the shift is 1, on the far diagonal 0, and
    // on edge 0 (the right side, outward +x) -1
    let [a, b, _] = triangles[0];
    assert_eq!(a.normal, [-1.0, -1.0]);
    assert_eq!(a.shift, [-1.0, 0.0, 1.0]);
    assert_eq!(b.normal, [1.0, -1.0]);
    assert_eq!(b.shift, [1.0, 0.0, 1.0]);
}

#[test]
fn a_corners_altitude_is_its_distance_to_the_opposite_edge() {
    // Given: a 3-4-5 triangle alone, every edge outline
    let points = [[0.0, 0.0], [4.0, 0.0], [0.0, 3.0]];

    // When
    let triangles = triangles_with_edges(&points, &[0, 1, 2]);

    // Then: corner 0 to the hypotenuse is 12 / 5
    let [a, b, c] = triangles[0];
    assert!((a.edges[0] - 2.4).abs() < 1e-6, "{:?}", a.edges);
    assert_eq!(b.edges, [0.0, 4.0, 0.0]);
    assert_eq!(c.edges, [0.0, 0.0, 3.0]);
    assert_eq!(check_mesh(&triangles), Ok(()));
}

#[test]
fn winding_does_not_change_the_distances() {
    // When
    let forward = triangles_with_edges(&SQUARE, &[0, 1, 2]);
    let backward = triangles_with_edges(&SQUARE, &[0, 2, 1]);

    // Then
    assert_eq!(forward[0][0].edges, [1.0, 0.0, 0.0]);
    assert_eq!(backward[0][0].edges, [1.0, 0.0, 0.0]);
}

#[test]
fn a_triangle_without_area_is_dropped() {
    // Given: three collinear points, and a repeated index
    let points = [[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]];
    let none: [[MeshVertex; 3]; 0] = [];

    // When, Then
    assert_eq!(triangles_with_edges(&points, &[0, 1, 2]), none);
    assert_eq!(triangles_with_edges(&SQUARE, &[0, 0, 1]), none);
}

#[test]
fn a_dropped_sliver_does_not_make_its_neighbours_edge_interior() {
    // Given: a sliver (0, 1, 2) along the top edge 0-2 of the triangle
    // (0, 2, 3); counted, the edge would be shared and far
    let points = [[0.0, 0.0], [2.0, 0.0], [4.0, 0.0], [4.0, 4.0]];

    // When
    let triangles = triangles_with_edges(&points, &[0, 1, 2, 0, 2, 3]);

    // Then: one triangle, every edge on the outline
    assert_eq!(triangles.len(), 1);
    let [a, b, c] = triangles[0];
    assert!(a.edges[0] != FAR_EDGE && b.edges[1] != FAR_EDGE && c.edges[2] != FAR_EDGE);
    assert!((c.edges[2] - 4.0).abs() < 1e-6, "{:?}", c.edges);
}

#[test]
fn a_sharp_corner_is_bevelled_at_the_miter_limit() {
    // Given: a 10 degree spike at the origin
    let points = [
        [0.0, 0.0],
        [10.0, 0.0],
        [10.0, 10.0f32.to_radians().tan() * 10.0],
    ];

    // When
    let [tip, ..] = triangles_with_edges(&points, &[0, 1, 2])[0];

    // Then: the full miter would be 1 / sin(5 degrees) = 11.5; clamped
    let len = (tip.normal[0].powi(2) + tip.normal[1].powi(2)).sqrt();
    assert!((len - MITER_LIMIT).abs() < 1e-5, "{len}");
}

#[test]
#[should_panic(expected = "4 indices is not a whole number of triangles")]
fn a_remainder_of_indices_is_a_producer_bug() {
    let _ = triangles_with_edges(&SQUARE, &[0, 1, 2, 3]);
}

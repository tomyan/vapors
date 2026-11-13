//! Edge distances from a shared-vertex mesh: outline edges found by
//! use count, altitudes, interior edges far, flat triangles dropped.

use super::*;

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
    assert_eq!(a.edges, [1.0, FAR, 0.0]);
    assert_eq!(b.position, [1.0, 0.0]);
    assert_eq!(b.edges, [0.0, FAR, 0.0]);
    assert_eq!(c.position, [1.0, 1.0]);
    assert_eq!(c.edges, [0.0, FAR, 1.0]);
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

    // When, Then
    let none: [[MeshVertex; 3]; 0] = [];
    assert_eq!(triangles_with_edges(&points, &[0, 1, 2]), none);
    assert_eq!(triangles_with_edges(&SQUARE, &[0, 0, 1]), none);
}

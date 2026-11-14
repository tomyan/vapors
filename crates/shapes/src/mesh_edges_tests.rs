//! From a shared-vertex mesh: outline edges by use count, altitudes,
//! interior edges far, flat triangles dropped before counting.

use super::*;
use crate::mesh::check_mesh;

/// A unit square as two triangles sharing the diagonal 0-2.
pub(crate) const SQUARE: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
pub(crate) const SQUARE_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];

/// The triangles of `indices`, which the test expects to be built.
pub(crate) fn built(vertices: &[[f32; 2]], indices: &[u32]) -> Vec<[MeshVertex; 3]> {
    triangles_with_edges(vertices, indices).unwrap_or_else(|e| panic!("{indices:?}: {e}"))
}

#[test]
fn a_squares_outer_edges_are_outline_and_its_diagonal_is_far() {
    // When
    let triangles = built(&SQUARE, &SQUARE_INDICES);

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
fn a_corners_altitude_is_its_distance_to_the_opposite_edge() {
    // Given: a 3-4-5 triangle alone, every edge outline
    let points = [[0.0, 0.0], [4.0, 0.0], [0.0, 3.0]];

    // When
    let triangles = built(&points, &[0, 1, 2]);

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
    let forward = built(&SQUARE, &[0, 1, 2]);
    let backward = built(&SQUARE, &[0, 2, 1]);

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
    assert_eq!(built(&points, &[0, 1, 2]), none);
    assert_eq!(built(&SQUARE, &[0, 0, 1]), none);
}

#[test]
fn a_dropped_sliver_does_not_make_its_neighbours_edge_interior() {
    // Given: a sliver (0, 1, 2) along the top edge 0-2 of the triangle
    // (0, 2, 3); counted, the edge would be shared and far
    let points = [[0.0, 0.0], [2.0, 0.0], [4.0, 0.0], [4.0, 4.0]];

    // When
    let triangles = built(&points, &[0, 1, 2, 0, 2, 3]);

    // Then: one triangle, every edge on the outline
    assert_eq!(triangles.len(), 1);
    let [a, b, c] = triangles[0];
    assert!(a.edges[0] != FAR_EDGE && b.edges[1] != FAR_EDGE && c.edges[2] != FAR_EDGE);
    assert!((c.edges[2] - 4.0).abs() < 1e-6, "{:?}", c.edges);
}

#[test]
#[should_panic(expected = "4 indices is not a whole number of triangles")]
fn a_remainder_of_indices_is_a_producer_bug() {
    let _ = triangles_with_edges(&SQUARE, &[0, 1, 2, 3]);
}

#[test]
fn copies_of_a_vertex_are_one_vertex_so_their_edge_is_shared() {
    // Given: the square with corner (0, 0) also at index 4, as lyon's
    // stroker re-creates a closed subpath's start; the second triangle
    // reaches the diagonal through the copy
    let mut points = SQUARE.to_vec();
    points.push([0.0, 0.0]);

    // When
    let triangles = built(&points, &[0, 1, 2, 4, 2, 3]);

    // Then: the diagonal is far in both triangles
    assert_eq!(triangles[0][0].edges[1], FAR_EDGE);
    assert_eq!(triangles[1][0].edges[2], FAR_EDGE);
    assert_eq!(check_mesh(&triangles), Ok(()));
}

#[test]
fn an_edge_three_triangles_use_is_refused_naming_it() {
    // Given: the square with its first triangle drawn twice, so the
    // diagonal 0-2 has three triangles
    let indices = [0, 1, 2, 0, 2, 3, 0, 2, 1];

    // When, Then
    assert_eq!(
        triangles_with_edges(&SQUARE, &indices),
        Err("the edge from [0.0, 0.0] to [1.0, 1.0] is used by 3 triangles, so the tessellation overlaps itself there; the tessellator does not draw that, so report the path as a fixture".to_owned())
    );
}

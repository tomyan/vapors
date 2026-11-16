//! From a shared-vertex mesh: outline edges by use count, each corner
//! bounded by the outline edges round its vertex, flat triangles
//! dropped before counting.

use super::*;
use crate::mesh::{CONVEX, MeshCorner};
use crate::mesh_check::check_mesh;

/// A unit square as two triangles sharing the diagonal 0-2.
pub(crate) const SQUARE: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
pub(crate) const SQUARE_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];

const UP: [f32; 2] = [0.0, -1.0];
const LEFT: [f32; 2] = [-1.0, 0.0];

/// The triangles of `indices`, which the test expects to be built.
pub(crate) fn built(vertices: &[[f32; 2]], indices: &[u32]) -> Vec<MeshTriangle> {
    mesh_triangles(vertices, indices).unwrap_or_else(|e| panic!("{indices:?}: {e}"))
}

/// A convex corner at `position` bounded by `normals`, whose edges run
/// `reaches` from it.
pub(crate) fn convex(position: [f32; 2], normals: [[f32; 2]; 2], reaches: [f32; 2]) -> MeshCorner {
    MeshCorner {
        position,
        normals,
        kind: CONVEX,
        reaches,
    }
}

#[test]
fn a_corner_across_a_shared_diagonal_is_bounded_by_the_outline_beyond_it() {
    // When
    let triangles = built(&SQUARE, &SQUARE_INDICES);

    // Then: corner (0, 0) of the first triangle has the bottom edge
    // (outward -y) on one side and, across the diagonal, the left edge
    // (outward -x) on the other
    assert_eq!(triangles.len(), 2);
    assert_eq!(
        triangles[0].corners[0],
        convex([0.0, 0.0], [UP, LEFT], [1.0, -1.0])
    );
    assert_eq!(check_mesh(&triangles), Ok(()));
}

#[test]
fn a_lone_triangles_corners_are_bounded_by_its_own_edges() {
    // Given: a 3-4-5 triangle alone, every edge outline
    let points = [[0.0, 0.0], [4.0, 0.0], [0.0, 3.0]];

    // When
    let triangles = built(&points, &[0, 1, 2]);

    // Then: the hypotenuse's outward normal is (3, 4) / 5
    let hypotenuse = [0.6, 0.8];
    let [a, b, c] = triangles[0].corners;
    assert_eq!(a, convex([0.0, 0.0], [UP, LEFT], [4.0, -3.0]));
    assert_eq!(b, convex([4.0, 0.0], [hypotenuse, UP], [5.0, -4.0]));
    assert_eq!(c, convex([0.0, 3.0], [LEFT, hypotenuse], [3.0, -5.0]));
    assert_eq!(check_mesh(&triangles), Ok(()));
}

#[test]
fn a_triangle_without_area_is_dropped() {
    // Given: three collinear points, and a repeated index
    let points = [[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]];

    // When, Then
    assert_eq!(built(&points, &[0, 1, 2]), []);
    assert_eq!(built(&SQUARE, &[0, 0, 1]), []);
}

#[test]
fn a_dropped_sliver_does_not_make_its_neighbours_edge_interior() {
    // Given: a sliver (0, 1, 2) along the bottom edge 0-2 of the
    // triangle (0, 2, 3); counted, the edge would be shared and inside
    let points = [[0.0, 0.0], [2.0, 0.0], [4.0, 0.0], [4.0, 4.0]];

    // When
    let triangles = built(&points, &[0, 1, 2, 0, 2, 3]);

    // Then: one triangle, its corner (4, 0) bounded by the bottom edge
    assert_eq!(triangles.len(), 1);
    assert_eq!(
        triangles[0].corners[1],
        convex([4.0, 0.0], [[1.0, 0.0], UP], [4.0, -4.0])
    );
}

#[test]
#[should_panic(expected = "4 indices is not a whole number of triangles")]
fn a_remainder_of_indices_is_a_producer_bug() {
    let _ = mesh_triangles(&SQUARE, &[0, 1, 2, 3]);
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

    // Then: the walk crosses the diagonal, as in the welded square
    assert_eq!(
        triangles[0].corners[0],
        convex([0.0, 0.0], [UP, LEFT], [1.0, -1.0])
    );
    assert_eq!(check_mesh(&triangles), Ok(()));
}

#[test]
fn an_edge_three_triangles_use_is_refused_naming_it() {
    // Given: the square with its first triangle drawn twice, so the
    // diagonal 0-2 has three triangles
    let indices = [0, 1, 2, 0, 2, 3, 0, 2, 1];

    // When, Then
    assert_eq!(
        mesh_triangles(&SQUARE, &indices),
        Err("the edge from [0.0, 0.0] to [1.0, 1.0] is used by 3 triangles, so the tessellation overlaps itself there; the tessellator does not draw that, so report the path as a fixture".to_owned())
    );
}

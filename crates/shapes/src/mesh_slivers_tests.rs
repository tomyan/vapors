//! A dropped sliver between triangles on both sides is a T-junction
//! inside the shape, refused naming the vertex. One along the outline
//! is dropped: `a_dropped_sliver_does_not_make_its_neighbours_edge_interior`
//! in `mesh_edges_tests.rs`.

use crate::mesh_edges::triangles_with_edges;

#[test]
fn a_sliver_between_triangles_on_both_sides_is_refused_naming_the_vertex() {
    // Given: (0, 2, 3) above the line y = 0 across its whole edge 0-2,
    // (0, 1, 4) and (1, 2, 4) below it meeting at (2, 0), and the
    // sliver (0, 1, 2) along it; dropped, its three edges would each be
    // used once and draw as outline inside the fill
    let points = [[0.0, 0.0], [2.0, 0.0], [4.0, 0.0], [2.0, 4.0], [2.0, -4.0]];

    // When
    let refused = triangles_with_edges(&points, &[0, 2, 3, 0, 1, 4, 1, 2, 4, 0, 1, 2]);

    // Then
    assert_eq!(
        refused,
        Err("triangles meet at a T-junction at [2.0, 0.0] inside the shape, across a triangle of no area from [0.0, 0.0] to [4.0, 0.0]; its edges would draw as outline, and the tessellator does not resolve this yet, so report the path as a fixture".to_owned())
    );
}

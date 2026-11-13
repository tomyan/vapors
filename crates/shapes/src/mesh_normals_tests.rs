//! Each corner's normal: the miter offset of its wedge's outline
//! edges with shift 1 on each, clamped at the miter limit; zero inside;
//! per corner where the outline touches itself; refused head-on.

use crate::mesh::check_mesh;
use crate::mesh_edges::tests::{SQUARE, SQUARE_INDICES, built};
use crate::mesh_edges::triangles_with_edges;

use super::*;

#[test]
fn a_corner_moves_along_the_miter_of_its_outline_edges_with_shift_one_on_each() {
    // When
    let triangles = built(&SQUARE, &SQUARE_INDICES);

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
fn a_sharp_corner_is_bevelled_at_the_miter_limit() {
    // Given: a 10 degree spike at the origin
    let points = [
        [0.0, 0.0],
        [10.0, 0.0],
        [10.0, 10.0f32.to_radians().tan() * 10.0],
    ];

    // When
    let [tip, ..] = built(&points, &[0, 1, 2])[0];

    // Then: the full miter would be 1 / sin(5 degrees) = 11.5; clamped
    let len = (tip.normal[0].powi(2) + tip.normal[1].powi(2)).sqrt();
    assert!((len - MITER_LIMIT).abs() < 1e-5, "{len}");
}

#[test]
fn a_vertex_the_triangles_surround_does_not_move() {
    // Given: a unit square fanned round its centre, vertex 4
    let points = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.5, 0.5]];

    // When
    let triangles = built(&points, &[4, 0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0]);

    // Then
    assert!(triangles.iter().all(|t| t[0].normal == [0.0, 0.0]));
    assert_eq!(check_mesh(&triangles), Ok(()));
}

#[test]
fn each_wedge_of_a_bowtie_moves_out_of_its_own_side() {
    // Given: two triangles touching at the origin, one opening to +x and
    // one to -x; the vertex has four outline edges whose normals cancel
    let points = [
        [0.0, 0.0],
        [2.0, -1.0],
        [2.0, 1.0],
        [-2.0, 1.0],
        [-2.0, -1.0],
    ];

    // When
    let triangles = built(&points, &[0, 1, 2, 0, 3, 4]);

    // Then: each corner at the origin takes the miter of its own two
    // edges, (-sqrt 5, 0) and (sqrt 5, 0)
    let (right, left) = (triangles[0][0].normal, triangles[1][0].normal);
    assert!(
        (right[0] + 5f32.sqrt()).abs() < 1e-5 && right[1].abs() < 1e-6,
        "{right:?}"
    );
    assert!(
        (left[0] - 5f32.sqrt()).abs() < 1e-5 && left[1].abs() < 1e-6,
        "{left:?}"
    );
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
    let refused = triangles_with_edges(&points, &[0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 5]);

    // Then
    assert_eq!(
        refused,
        Err("outline edges meet head-on at [0.0, 0.0], a spike of no width with no direction to move out in; remove the spike".to_owned())
    );
}

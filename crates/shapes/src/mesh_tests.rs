//! The mesh format: its layout, and which of a triangle's edges are on
//! the outline.

use super::*;

#[test]
fn the_layout_is_position_normals_kind_reaches_three_corners_a_triangle() {
    assert_eq!(size_of::<MeshCorner>(), 36);
    assert_eq!(std::mem::offset_of!(MeshCorner, normals), 8);
    assert_eq!(std::mem::offset_of!(MeshCorner, kind), 24);
    assert_eq!(std::mem::offset_of!(MeshCorner, reaches), 28);
    assert_eq!(size_of::<MeshTriangle>(), 108);
}

#[test]
fn an_edge_is_on_the_outline_when_both_its_ends_bound_by_it() {
    // Given: a unit square's triangle (a b c), each corner bounded by
    // the sides meeting there, and a copy whose corner c no longer
    // bounds its wedge by the right side (outward +x)
    let corner = |position, normals| MeshCorner {
        position,
        normals,
        kind: CONVEX,
        reaches: [1.0, 1.0],
    };
    let (up, right, down, left) = ([0.0, -1.0], [1.0, 0.0], [0.0, 1.0], [-1.0, 0.0]);
    let triangle = MeshTriangle {
        corners: [
            corner([0.0, 0.0], [left, up]),
            corner([1.0, 0.0], [up, right]),
            corner([1.0, 1.0], [right, down]),
        ],
    };
    let mut one_end = triangle;
    one_end.corners[2].normals = [down, down];

    // When, Then: edges opposite a, b, c are b-c (right side), the
    // diagonal c-a, and a-b (top)
    assert_eq!(
        [0, 1, 2].map(|edge| triangle.on_outline(edge)),
        [true, false, true]
    );
    assert!(!one_end.on_outline(0));
}

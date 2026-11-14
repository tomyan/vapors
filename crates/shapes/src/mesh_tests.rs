//! The mesh rule checked: a worked square of two triangles passes;
//! each wrong number is refused naming triangle, corner and field.

use super::*;

/// A corner: its position, normal, edge distances and shifts.
const fn v(position: [f32; 2], normal: [f32; 2], edges: [f32; 3], shift: [f32; 3]) -> MeshVertex {
    MeshVertex {
        position,
        normal,
        edges,
        shift,
    }
}

const F: f32 = FAR_EDGE;

/// A unit square as two triangles (0 1 2) and (0 2 3) sharing the
/// diagonal 0-2; each corner's normal is its miter offset (1, 1) scaled
/// for the right angle, so its shift is 1 on its own outline edges.
/// Triangle (0, 1, 2): edge 0 is 1-2 (outline, normal +x), edge 1 is
/// 2-0 (the diagonal, far), edge 2 is 0-1 (outline, normal -y).
fn square() -> Vec<[MeshVertex; 3]> {
    let (a, b, c, d) = ([0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]);
    vec![
        [
            v(a, [-1.0, -1.0], [1.0, F, 0.0], [-1.0, 0.0, 1.0]),
            v(b, [1.0, -1.0], [0.0, F, 0.0], [1.0, 0.0, 1.0]),
            v(c, [1.0, 1.0], [0.0, F, 1.0], [1.0, 0.0, -1.0]),
        ],
        [
            v(a, [-1.0, -1.0], [1.0, 0.0, F], [-1.0, 1.0, 0.0]),
            v(c, [1.0, 1.0], [0.0, 1.0, F], [1.0, -1.0, 0.0]),
            v(d, [-1.0, 1.0], [0.0, 0.0, F], [1.0, 1.0, 0.0]),
        ],
    ]
}

fn refused(triangles: &[[MeshVertex; 3]], reason: &str) {
    assert_eq!(
        check_mesh(triangles).as_ref().map_err(String::as_str),
        Err(reason)
    );
}

#[test]
fn a_square_of_two_triangles_with_its_diagonal_far_passes() {
    assert_eq!(check_mesh(&square()), Ok(()));
}

#[test]
fn the_layout_is_position_normal_edges_shift() {
    assert_eq!(size_of::<MeshVertex>(), 40);
    assert_eq!(std::mem::offset_of!(MeshVertex, shift), 28);
}

#[test]
fn a_mesh_with_no_triangles_is_refused() {
    refused(&[], "a mesh with no triangles draws nothing");
}

#[test]
fn a_shared_edge_far_on_two_corners_only_would_seam_and_is_refused() {
    // Given: the diagonal at 0 on its two corners, as the one-scalar
    // rule would write it
    let mut mesh = square();
    mesh[0][0].edges[1] = 0.0;
    mesh[0][2].edges[1] = 0.0;

    // When, Then
    refused(
        &mesh,
        "triangle 0 edge 1 is far on corners [false, true, false] but an edge is far on all three or none",
    );
}

#[test]
fn a_distance_that_the_positions_do_not_give_is_refused() {
    // Given: corner 0's altitude to edge 0 written as 2.8 on a square of
    // side 1
    let mut mesh = square();
    mesh[0][0].edges[0] = 2.8;

    // When, Then
    refused(
        &mesh,
        "triangle 0 corner 0 distance 2.8 to edge 0 should be 1 from the positions",
    );
}

#[test]
fn a_shift_that_is_not_the_normals_projection_is_refused() {
    // Given: corner 1 moves along +x, -y; its shift on edge 0 (normal
    // +x) must be 1
    let mut mesh = square();
    mesh[0][1].shift[0] = 0.5;

    // When, Then
    refused(
        &mesh,
        "triangle 0 corner 1 shift 0.5 to edge 0 should be 1 from the positions",
    );
}

#[test]
fn a_shift_on_a_far_edge_is_refused() {
    // Given
    let mut mesh = square();
    mesh[1][2].shift[2] = 1.0;

    // When, Then
    refused(
        &mesh,
        "triangle 1 corner 2 shift 1 on far edge 2 is not 0; a far edge stays far",
    );
}

#[test]
fn a_value_that_is_not_finite_is_refused_naming_the_field() {
    // Given
    let mut nan_position = square();
    nan_position[1][1].position = [f32::NAN, 1.0];
    let mut infinite_edge = square();
    infinite_edge[0][2].edges = [0.0, F, f32::INFINITY];
    let mut nan_normal = square();
    nan_normal[0][0].normal = [f32::NAN, 0.0];

    // When, Then
    refused(
        &nan_position,
        "triangle 1 corner 1 position [NaN, 1.0] is not finite",
    );
    refused(
        &infinite_edge,
        "triangle 0 corner 2 edges [0.0, 1000000.0, inf] is not finite",
    );
    refused(
        &nan_normal,
        "triangle 0 corner 0 normal [NaN, 0.0] is not finite",
    );
}

#[test]
fn a_triangle_without_area_is_refused() {
    // Given
    let mut mesh = square();
    mesh[0][2].position = [2.0, 0.0];

    // When, Then
    refused(
        &mesh,
        "triangle 0 has no area: corners [[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]]",
    );
}

#[test]
fn a_corner_on_the_outline_that_does_not_move_out_is_refused() {
    // Given: corner 1 with no normal and so no shift, which would leave
    // the hard outer edge the extrusion is for
    let mut zero = square();
    zero[0][1].normal = [0.0, 0.0];
    zero[0][1].shift = [0.0, 0.0, 0.0];
    // and corner 1 moving inwards, its shifts its projection
    let mut inward = square();
    inward[0][1].normal = [-1.0, 1.0];
    inward[0][1].shift = [-1.0, 0.0, -1.0];

    // When, Then
    refused(
        &zero,
        "triangle 0 corner 1 on outline edge 0 has normal [0.0, 0.0], which does not move it out across that edge; write its miter offset",
    );
    refused(
        &inward,
        "triangle 0 corner 1 on outline edge 0 has normal [-1.0, 1.0], which does not move it out across that edge; write its miter offset",
    );
}

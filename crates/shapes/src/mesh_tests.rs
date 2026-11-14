//! The mesh rule checked: a worked square of two triangles passes;
//! each wrong number is refused naming triangle, corner and field.

use super::*;

const fn corner(position: [f32; 2], normals: [[f32; 2]; 2]) -> MeshCorner {
    MeshCorner {
        position,
        normals,
        kind: CONVEX,
    }
}

/// A unit square as two triangles (a b c) and (a c d) sharing the
/// diagonal a-c; every corner is convex, its wedge bounded by the two
/// sides that meet there (outward -y, +x, +y, -x along a-b, b-c, c-d,
/// d-a).
fn square() -> Vec<MeshTriangle> {
    let (up, right, down, left) = ([0.0, -1.0], [1.0, 0.0], [0.0, 1.0], [-1.0, 0.0]);
    let a = corner([0.0, 0.0], [left, up]);
    let b = corner([1.0, 0.0], [up, right]);
    let c = corner([1.0, 1.0], [right, down]);
    let d = corner([0.0, 1.0], [down, left]);
    vec![
        MeshTriangle { corners: [a, b, c] },
        MeshTriangle { corners: [a, c, d] },
    ]
}

fn refused(triangles: &[MeshTriangle], reason: &str) {
    assert_eq!(
        check_mesh(triangles).as_ref().map_err(String::as_str),
        Err(reason)
    );
}

#[test]
fn a_square_of_two_triangles_with_convex_corners_passes() {
    assert_eq!(check_mesh(&square()), Ok(()));
}

#[test]
fn the_layout_is_position_normals_kind_three_corners_a_triangle() {
    assert_eq!(size_of::<MeshCorner>(), 28);
    assert_eq!(std::mem::offset_of!(MeshCorner, normals), 8);
    assert_eq!(std::mem::offset_of!(MeshCorner, kind), 24);
    assert_eq!(size_of::<MeshTriangle>(), 84);
}

#[test]
fn a_mesh_with_no_triangles_is_refused() {
    refused(&[], "a mesh with no triangles draws nothing");
}

#[test]
fn a_value_that_is_not_finite_is_refused_naming_the_field() {
    // Given
    let mut nan_position = square();
    nan_position[1].corners[1].position = [f32::NAN, 1.0];
    let mut infinite_normal = square();
    infinite_normal[0].corners[2].normals[1] = [0.0, f32::INFINITY];
    let mut nan_kind = square();
    nan_kind[0].corners[0].kind = f32::NAN;

    // When, Then
    refused(
        &nan_position,
        "triangle 1 corner 1 position [NaN, 1.0] is not finite",
    );
    refused(
        &infinite_normal,
        "triangle 0 corner 2 normals [1.0, 0.0, 0.0, inf] is not finite",
    );
    refused(&nan_kind, "triangle 0 corner 0 kind [NaN] is not finite");
}

#[test]
fn a_triangle_without_area_is_refused() {
    // Given
    let mut mesh = square();
    mesh[0].corners[2].position = [2.0, 0.0];

    // When, Then
    refused(
        &mesh,
        "triangle 0 has no area: corners [[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]]",
    );
}

#[test]
fn a_kind_that_is_none_of_the_three_is_refused() {
    // Given
    let mut mesh = square();
    mesh[1].corners[2].kind = 0.5;

    // When, Then
    refused(
        &mesh,
        "triangle 1 corner 2 has kind 0.5, not convex (1), reflex (-1) or inside (0)",
    );
}

#[test]
fn an_inside_corner_with_normals_is_refused() {
    // Given: corner a marked inside but keeping its wedge
    let mut mesh = square();
    mesh[0].corners[0].kind = INSIDE;

    // When, Then
    refused(
        &mesh,
        "triangle 0 corner 0 is inside but has normals [[-1.0, 0.0], [0.0, -1.0]]; an inside corner has none",
    );
}

#[test]
fn a_normal_that_is_not_unit_length_is_refused() {
    // Given
    let mut mesh = square();
    mesh[0].corners[1].normals[1] = [0.5, 0.0];

    // When, Then
    refused(
        &mesh,
        "triangle 0 corner 1 has normal [0.5, 0.0], of length 0.5, not a unit normal",
    );
}

#[test]
fn normals_that_meet_head_on_are_refused() {
    // Given
    let mut mesh = square();
    mesh[0].corners[1].normals = [[0.0, -1.0], [0.0, 1.0]];

    // When, Then
    refused(
        &mesh,
        "triangle 0 corner 1 has normals [[0.0, -1.0], [0.0, 1.0]] that meet head-on; its wedge has no inside",
    );
}

#[test]
fn a_triangle_outside_its_corners_convex_wedge_is_refused() {
    // Given: corner a's wedge turned to face +x, away from the square
    let mut mesh = square();
    mesh[0].corners[0].normals = [[0.0, -1.0], [1.0, 0.0]];

    // When, Then
    refused(
        &mesh,
        "triangle 0 corner 0 has a wedge whose normals [[0.0, -1.0], [1.0, 0.0]] put the triangle's corner [1.0, 0.0] outside it",
    );
}

#[test]
fn the_same_wedge_holds_the_triangle_when_reflex() {
    // Given: the wedge above, which holds the triangle inside either line
    let mut mesh = square();
    mesh[0].corners[0].normals = [[0.0, -1.0], [1.0, 0.0]];
    mesh[0].corners[0].kind = REFLEX;

    // When, Then
    assert_eq!(check_mesh(&mesh), Ok(()));
}

#[test]
fn an_edge_is_on_the_outline_when_both_its_ends_bound_by_it() {
    // Given: the square's first triangle, and a copy whose corner c no
    // longer bounds its wedge by the right side (outward +x)
    let triangle = square()[0];
    let mut one_end = triangle;
    one_end.corners[2].normals = [[0.0, 1.0], [0.0, 1.0]];

    // When, Then: edges opposite a, b, c are b-c (right side), the
    // diagonal c-a, and a-b (top)
    assert_eq!(
        [0, 1, 2].map(|edge| triangle.on_outline(edge)),
        [true, false, true]
    );
    assert!(!one_end.on_outline(0));
}

//! Each geometry the check refuses, and the ones it passes.

use super::*;

const V: MeshVertex = MeshVertex {
    position: [0.0, 0.0],
    outline_distance: 0.0,
};
const TRIANGLE: [MeshVertex; 3] = [
    V,
    MeshVertex {
        position: [4.0, 0.0],
        outline_distance: 0.0,
    },
    MeshVertex {
        position: [0.0, 4.0],
        outline_distance: 0.0,
    },
];

fn rect(size: [f32; 2], radius: [f32; 2]) -> Geometry {
    Geometry::Rect {
        origin: [1.0, 1.0],
        size,
        radius,
    }
}

fn ellipse(centre: [f32; 2], radii: [f32; 2]) -> Geometry {
    Geometry::Ellipse { centre, radii }
}

fn refused(geometry: &Geometry, reason: &str) {
    assert_eq!(
        check_geometry(geometry).as_ref().map_err(String::as_str),
        Err(reason),
        "{geometry:?}"
    );
}

#[test]
fn drawable_geometries_pass() {
    // Given
    let drawable = [
        rect([2.0, 3.0], [0.0, 0.0]),
        rect([2.0, 3.0], [1.0, 1.0]),
        ellipse([8.0, 8.0], [3.0, 3.0]),
        Geometry::Mesh {
            vertices: &TRIANGLE,
            indices: &[0, 1, 2],
        },
    ];

    // When, Then
    for geometry in &drawable {
        assert_eq!(check_geometry(geometry), Ok(()), "{geometry:?}");
    }
}

#[test]
fn a_closed_form_with_a_non_finite_or_non_positive_value_is_refused() {
    // Given
    let cases = [
        (
            rect([f32::NAN, 1.0], [0.0; 2]),
            "size [NaN, 1.0] is not a finite number",
        ),
        (
            rect([2.0, 0.0], [0.0; 2]),
            "size [2.0, 0.0] is not above zero",
        ),
        (
            Geometry::Rect {
                origin: [f32::INFINITY, 0.0],
                size: [1.0; 2],
                radius: [0.0; 2],
            },
            "origin [inf, 0.0] is not a finite number",
        ),
        (
            rect([2.0, 2.0], [f32::NAN, f32::NAN]),
            "radius [NaN, NaN] is not a finite number",
        ),
        (
            ellipse([f32::NEG_INFINITY, 0.0], [1.0; 2]),
            "centre [-inf, 0.0] is not a finite number",
        ),
        (
            ellipse([0.0; 2], [1.0, -1.0]),
            "radii [1.0, -1.0] is not above zero",
        ),
    ];

    // When, Then
    for (geometry, reason) in &cases {
        refused(geometry, reason);
    }
}

#[test]
fn a_corner_radius_the_box_shader_cannot_draw_is_refused() {
    // Given
    let cases = [
        (
            rect([2.0, 2.0], [-1.0, -1.0]),
            "a corner radius of [-1.0, -1.0] is negative",
        ),
        (
            rect([10.0, 4.0], [6.0, 6.0]),
            "a corner radius of [6.0, 6.0] is over half the size [10.0, 4.0]; the emitter clamps rx to width / 2 and ry to height / 2, as SVG does",
        ),
        (
            rect([10.0, 4.0], [1.0, 2.5]),
            "a corner radius of [1.0, 2.5] is over half the size [10.0, 4.0]; the emitter clamps rx to width / 2 and ry to height / 2, as SVG does",
        ),
        (
            rect([2.0, 3.0], [0.5, 1.0]),
            "corner radii [0.5, 1.0] differ, and the box shader draws circular corners; elliptical corners come with S3a",
        ),
    ];

    // When, Then
    for (geometry, reason) in &cases {
        refused(geometry, reason);
    }
}

#[test]
fn a_mesh_whose_indices_do_not_name_triangles_of_its_vertices_is_refused() {
    // Given
    let cases: [(&'static [MeshVertex], &'static [u32], &str); 4] = [
        (
            &[],
            &[0, 1, 2],
            "0 vertices and 3 indices; a mesh names at least one triangle",
        ),
        (
            &TRIANGLE,
            &[],
            "3 vertices and 0 indices; a mesh names at least one triangle",
        ),
        (
            &TRIANGLE,
            &[0, 1],
            "2 indices is not a whole number of triangles",
        ),
        (
            &TRIANGLE,
            &[0, 1, 3],
            "index 3 at position 2 names no vertex; the mesh has 3",
        ),
    ];

    // When, Then
    for (vertices, indices, reason) in cases {
        refused(&Geometry::Mesh { vertices, indices }, reason);
    }
}

#[test]
fn a_mesh_with_a_non_finite_vertex_is_refused_naming_its_position() {
    // Given
    static DISTANCE: [MeshVertex; 3] = [
        V,
        V,
        MeshVertex {
            position: [1.0, 1.0],
            outline_distance: f32::NAN,
        },
    ];
    static POSITION: [MeshVertex; 3] = [
        V,
        MeshVertex {
            position: [f32::INFINITY, 1.0],
            outline_distance: 0.0,
        },
        V,
    ];

    // When, Then
    refused(
        &Geometry::Mesh {
            vertices: &DISTANCE,
            indices: &[0, 1, 2],
        },
        "vertex 2 MeshVertex { position: [1.0, 1.0], outline_distance: NaN } is not finite",
    );
    refused(
        &Geometry::Mesh {
            vertices: &POSITION,
            indices: &[0, 1, 2],
        },
        "vertex 1 MeshVertex { position: [inf, 1.0], outline_distance: 0.0 } is not finite",
    );
}

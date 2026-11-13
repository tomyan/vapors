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

#[test]
fn drawable_geometries_pass() {
    // Given
    let drawable = [
        rect([2.0, 3.0], [0.0, 0.0]),
        rect([2.0, 3.0], [0.5, 0.5]),
        Geometry::Ellipse {
            centre: [8.0, 8.0],
            radii: [3.0, 3.0],
        },
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
    let refused = [
        (
            rect([f32::NAN, 1.0], [0.0; 2]),
            "size [NaN, 1.0] is not a finite number",
        ),
        (
            rect([2.0, 0.0], [0.0; 2]),
            "size [2.0, 0.0] is not above zero",
        ),
        (
            rect([2.0, 2.0], [-1.0, 0.0]),
            "a corner radius of [-1.0, 0.0] is negative",
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
            Geometry::Ellipse {
                centre: [0.0; 2],
                radii: [1.0, -1.0],
            },
            "radii [1.0, -1.0] is not above zero",
        ),
    ];

    // When, Then
    for (geometry, reason) in &refused {
        assert_eq!(
            check_geometry(geometry).as_ref().map_err(String::as_str),
            Err(*reason)
        );
    }
}

#[test]
fn a_mesh_whose_indices_do_not_name_triangles_of_its_vertices_is_refused() {
    // Given
    let refused: [(&'static [MeshVertex], &'static [u32], &str); 4] = [
        (
            &[],
            &[0, 1, 2],
            "a mesh with no vertices or no indices draws nothing",
        ),
        (
            &TRIANGLE,
            &[],
            "a mesh with no vertices or no indices draws nothing",
        ),
        (
            &TRIANGLE,
            &[0, 1],
            "2 indices is not a whole number of triangles",
        ),
        (
            &TRIANGLE,
            &[0, 1, 3],
            "index 3 names no vertex; the mesh has 3",
        ),
    ];

    // When, Then
    for (vertices, indices, reason) in refused {
        let geometry = Geometry::Mesh { vertices, indices };
        assert_eq!(
            check_geometry(&geometry).as_ref().map_err(String::as_str),
            Err(reason)
        );
    }
}

#[test]
fn a_mesh_with_a_non_finite_vertex_is_refused() {
    // Given
    static VERTICES: [MeshVertex; 3] = [
        V,
        V,
        MeshVertex {
            position: [1.0, 1.0],
            outline_distance: f32::NAN,
        },
    ];
    let geometry = Geometry::Mesh {
        vertices: &VERTICES,
        indices: &[0, 1, 2],
    };

    // When
    let result = check_geometry(&geometry);

    // Then
    assert_eq!(
        result.as_ref().map_err(String::as_str),
        Err("vertex MeshVertex { position: [1.0, 1.0], outline_distance: NaN } is not finite")
    );
}

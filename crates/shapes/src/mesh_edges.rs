//! From lyon's shared-vertex output to the triangles `mesh.rs` checks:
//! each with its own three vertices, the edge distances by the rule
//! there, and the normal (`mesh_normals.rs`) and shifts the half-pixel
//! extrusion needs. An edge used by one surviving triangle is on the
//! outline; an edge two share is interior and far. Triangles without
//! area are dropped first, so a sliver along the outline cannot make
//! its neighbour's edge count as shared.

use std::collections::HashMap;

use crate::mesh::{FAR_EDGE, MeshVertex, length, outward_normal, twice_area};
use crate::mesh_normals::corner_normals;

/// The triangles using each undirected edge, by index; two at most for
/// a mesh this code understands.
pub(crate) type EdgeTriangles = HashMap<(u32, u32), Vec<usize>>;

/// The triangles of `indices` over `vertices`, three indices each.
///
/// # Errors
/// Outline edges meeting head-on at a vertex (`mesh_normals.rs`).
///
/// # Panics
/// When `indices` is not a whole number of triangles: lyon emits
/// triples, so a remainder is a producer bug.
pub fn triangles_with_edges(
    vertices: &[[f32; 2]],
    indices: &[u32],
) -> Result<Vec<[MeshVertex; 3]>, String> {
    assert!(
        indices.len().is_multiple_of(3),
        "{} indices is not a whole number of triangles; the producer must emit triples",
        indices.len()
    );
    let corners: Vec<[u32; 3]> = indices
        .as_chunks::<3>()
        .0
        .iter()
        .copied()
        .filter(|corner| twice_area(positions(vertices, *corner)) != 0.0)
        .collect();
    let edges = edge_triangles(&corners);
    let normals = corner_normals(vertices, &corners, &edges)?;
    Ok(corners
        .iter()
        .zip(normals)
        .map(|(corner, normals)| {
            let outline =
                [1, 2, 0].map(|k| edges[&edge_key(corner[k], corner[(k + 1) % 3])].len() == 1);
            triangle(positions(vertices, *corner), normals, outline)
        })
        .collect())
}

fn positions(vertices: &[[f32; 2]], corner: [u32; 3]) -> [[f32; 2]; 3] {
    corner.map(|i| vertices[i as usize])
}

fn edge_triangles(corners: &[[u32; 3]]) -> EdgeTriangles {
    let mut edges = EdgeTriangles::with_capacity(corners.len() * 3);
    for (t, corner) in corners.iter().enumerate() {
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            let users = edges.entry(edge_key(corner[a], corner[b])).or_default();
            users.push(t);
            debug_assert!(
                users.len() <= 2,
                "edge {:?} is used by {} triangles; the producer overlaps itself",
                edge_key(corner[a], corner[b]),
                users.len()
            );
        }
    }
    edges
}

pub(crate) fn edge_key(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

/// The triangle over `points` with its corners' `normals`: edge `i`
/// opposite corner `i`, distances and shifts by the rule in `mesh.rs`.
fn triangle(points: [[f32; 2]; 3], normals: [[f32; 2]; 3], outline: [bool; 3]) -> [MeshVertex; 3] {
    let area2 = twice_area(points).abs();
    let mut out = [MeshVertex {
        position: [0.0; 2],
        normal: [0.0; 2],
        edges: [FAR_EDGE; 3],
        shift: [0.0; 3],
    }; 3];
    for (corner, vertex) in out.iter_mut().enumerate() {
        vertex.position = points[corner];
        vertex.normal = normals[corner];
        for (edge, on_outline) in outline.iter().enumerate() {
            if !on_outline {
                continue;
            }
            let (b, c) = (points[(edge + 1) % 3], points[(edge + 2) % 3]);
            vertex.edges[edge] = if edge == corner {
                area2 / length(b, c)
            } else {
                0.0
            };
            let n = outward_normal(points, edge);
            vertex.shift[edge] = normals[corner][0] * n[0] + normals[corner][1] * n[1];
        }
    }
    out
}

#[cfg(test)]
#[path = "mesh_edges_tests.rs"]
pub(crate) mod tests;

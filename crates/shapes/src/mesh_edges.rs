//! From lyon's shared-vertex output to the triangles `mesh.rs` checks:
//! each with its own three vertices, the edge distances by the rule
//! there, and the normal (`mesh_normals.rs`) and shifts the half-pixel
//! extrusion needs. Vertices at one position are welded first. An edge
//! used by one surviving triangle is on the outline; an edge two share
//! is interior and far. Triangles without area are dropped first, so a
//! sliver along the outline cannot make its neighbour's edge count as
//! shared.

use std::collections::HashMap;

use crate::mesh::{FAR_EDGE, MeshVertex, length, outward_normal, twice_area};
use crate::mesh_normals::corner_normals;
use crate::mesh_slivers::check_slivers;

/// The triangles using each undirected edge, by index; two at most,
/// as `edge_triangles` checks.
pub(crate) type EdgeTriangles = HashMap<(u32, u32), Vec<usize>>;

/// The triangles of `indices` over `vertices`, three indices each.
///
/// # Errors
/// An edge used by three or more triangles; a dropped sliver that is a
/// T-junction inside the shape (`mesh_slivers.rs`); outline edges
/// meeting head-on at a vertex (`mesh_normals.rs`).
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
    let indices = welded(vertices, indices);
    let (corners, slivers): (Vec<[u32; 3]>, Vec<[u32; 3]>) = indices
        .as_chunks::<3>()
        .0
        .iter()
        .copied()
        .partition(|corner| twice_area(positions(vertices, *corner)) != 0.0);
    let edges = edge_triangles(vertices, &corners)?;
    check_slivers(vertices, &slivers, &edges)?;
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

/// `indices` with each vertex replaced by the first at a bit-equal
/// position: lyon's stroker re-creates a closed subpath's start
/// vertices, and the edge across the stroke there, under two pairs of
/// indices, would count as two outline edges and draw a seam. The
/// tessellator does not read the stroke advancement the copies are for.
fn welded(vertices: &[[f32; 2]], indices: &[u32]) -> Vec<u32> {
    let mut first = HashMap::with_capacity(vertices.len());
    let canonical: Vec<u32> = (0u32..)
        .zip(vertices)
        .map(|(i, p)| *first.entry(p.map(|c| (c + 0.0).to_bits())).or_insert(i))
        .collect();
    indices.iter().map(|&i| canonical[i as usize]).collect()
}

fn positions(vertices: &[[f32; 2]], corner: [u32; 3]) -> [[f32; 2]; 3] {
    corner.map(|i| vertices[i as usize])
}

/// Each edge's triangles.
///
/// # Errors
/// An edge with three or more: the tessellation overlaps itself, which
/// the normals' walk round a vertex cannot follow.
fn edge_triangles(vertices: &[[f32; 2]], corners: &[[u32; 3]]) -> Result<EdgeTriangles, String> {
    let mut edges = EdgeTriangles::with_capacity(corners.len() * 3);
    for (t, corner) in corners.iter().enumerate() {
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            let users = edges.entry(edge_key(corner[a], corner[b])).or_default();
            users.push(t);
            if users.len() > 2 {
                return Err(format!(
                    "the edge from {:?} to {:?} is used by {} triangles, so the tessellation overlaps itself there; the tessellator does not draw that, so report the path as a fixture",
                    vertices[corner[a] as usize],
                    vertices[corner[b] as usize],
                    users.len()
                ));
            }
        }
    }
    Ok(edges)
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

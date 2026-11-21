//! From lyon's shared-vertex output to the triangles `mesh.rs` checks:
//! each with its three corners' wedges (`mesh_normals.rs`). Vertices at
//! one position are welded first. An edge used by one surviving
//! triangle is on the outline; an edge two share is interior.
//! Triangles without area are dropped first, so a sliver along the
//! outline cannot make its neighbour's edge count as shared.

use std::collections::HashMap;

use crate::mesh::{MeshTriangle, length, twice_area};
use crate::mesh_normals::corner_wedges;
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
pub fn mesh_triangles(vertices: &[[f32; 2]], indices: &[u32]) -> Result<Vec<MeshTriangle>, String> {
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
    let wedges = corner_wedges(vertices, &corners, &edges)?;
    Ok(wedges
        .into_iter()
        .map(|corners| MeshTriangle { corners })
        .collect())
}

/// `indices` with each vertex replaced by the first within `WELD` of
/// it (bit-equal included): lyon's stroker re-creates a closed
/// subpath's start vertices, and the edge across the stroke there,
/// under two pairs of indices, would count as two outline edges and
/// draw a seam; its flattened arc ends a rounding step off the point
/// it was told (a closed path's start), leaving a sliver whose edge of
/// that length has a noise for a direction. The tessellator does not
/// read the stroke advancement the copies are for.
fn welded(vertices: &[[f32; 2]], indices: &[u32]) -> Vec<u32> {
    let mut cells: HashMap<[i64; 2], Vec<u32>> = HashMap::with_capacity(vertices.len());
    let canonical: Vec<u32> = (0u32..)
        .zip(vertices)
        .map(|(i, p)| {
            let first = first_within(vertices, &cells, *p);
            first.unwrap_or_else(|| {
                cells.entry(cell(*p)).or_default().push(i);
                i
            })
        })
        .collect();
    indices.iter().map(|&i| canonical[i as usize]).collect()
}

/// The grid cell of `p`, `WELD` wide.
#[allow(clippy::cast_possible_truncation)] // a cell index of a finite coordinate
fn cell(p: [f32; 2]) -> [i64; 2] {
    p.map(|c| (c / WELD).floor() as i64)
}

/// The first kept vertex within `WELD` of `p`, looking in its cell and
/// the eight round it.
fn first_within(
    vertices: &[[f32; 2]],
    cells: &HashMap<[i64; 2], Vec<u32>>,
    p: [f32; 2],
) -> Option<u32> {
    let [x, y] = cell(p);
    let near = (x - 1..=x + 1).flat_map(|x| (y - 1..=y + 1).map(move |y| [x, y]));
    let kept = near.filter_map(|key| cells.get(&key)).flatten();
    kept.copied()
        .find(|&j| length(vertices[j as usize], p) <= WELD)
}

/// How close two vertices are to be one: a few steps of an `f32` at a
/// dashboard's coordinates, which no real edge is as short as.
const WELD: f32 = 1e-5;

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

#[cfg(test)]
#[path = "mesh_edges_tests.rs"]
pub(crate) mod tests;

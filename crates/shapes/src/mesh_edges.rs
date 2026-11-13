//! Each triangle's vertices with their distances to its three edges,
//! from a shared-vertex mesh: an edge used by one triangle alone is on
//! the outline, and a fragment's interpolated distance to it is its
//! distance to the outline there; an edge two triangles share is
//! interior, and its distance is [`FAR`] so it never limits coverage.
//! Triangles without area are dropped: they draw nothing.

use std::collections::HashMap;

use crate::geometry::MeshVertex;

/// The distance written for an interior edge: past any viewBox.
pub const FAR: f32 = 1.0e6;

/// The triangles of `indices` over `vertices` (three indices each),
/// each vertex carrying its distance to the triangle's edges.
#[must_use]
pub fn triangles_with_edges(vertices: &[[f32; 2]], indices: &[u32]) -> Vec<[MeshVertex; 3]> {
    let uses = edge_uses(indices);
    indices
        .as_chunks::<3>()
        .0
        .iter()
        .filter_map(|corner| {
            let outline = [
                uses[&edge_key(corner[1], corner[2])] == 1,
                uses[&edge_key(corner[2], corner[0])] == 1,
                uses[&edge_key(corner[0], corner[1])] == 1,
            ];
            let points = [
                vertices[corner[0] as usize],
                vertices[corner[1] as usize],
                vertices[corner[2] as usize],
            ];
            triangle(points, outline)
        })
        .collect()
}

/// How many triangles use each undirected edge.
fn edge_uses(indices: &[u32]) -> HashMap<(u32, u32), u32> {
    let mut uses = HashMap::with_capacity(indices.len());
    for corner in indices.as_chunks::<3>().0 {
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            *uses.entry(edge_key(corner[a], corner[b])).or_insert(0) += 1;
        }
    }
    uses
}

fn edge_key(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

/// The triangle over `points`, edge `i` opposite corner `i`, with each
/// corner's distances: zero to the two edges it is on, its altitude to
/// the opposite edge, and [`FAR`] to an edge not on the outline. `None`
/// when the triangle has no area.
fn triangle(points: [[f32; 2]; 3], outline: [bool; 3]) -> Option<[MeshVertex; 3]> {
    let area2 = twice_area(points);
    if area2 == 0.0 {
        return None;
    }
    let altitude = |corner: usize| {
        let (b, c) = (points[(corner + 1) % 3], points[(corner + 2) % 3]);
        area2.abs() / length(b, c)
    };
    let mut out = [MeshVertex {
        position: [0.0; 2],
        edges: [FAR; 3],
    }; 3];
    for (corner, vertex) in out.iter_mut().enumerate() {
        vertex.position = points[corner];
        for (edge, on_outline) in outline.iter().enumerate() {
            if *on_outline {
                vertex.edges[edge] = if edge == corner {
                    altitude(corner)
                } else {
                    0.0
                };
            }
        }
    }
    Some(out)
}

fn twice_area([a, b, c]: [[f32; 2]; 3]) -> f32 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn length(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

#[cfg(test)]
#[path = "mesh_edges_tests.rs"]
mod tests;

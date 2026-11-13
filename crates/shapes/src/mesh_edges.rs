//! From lyon's shared-vertex output to the triangles `mesh.rs` checks:
//! each with its own three vertices, the edge distances by the rule
//! there, and the normal and shifts the half-pixel extrusion needs.
//! An edge used by one surviving triangle is on the outline; an edge
//! two share is interior and far. Triangles without area are dropped
//! first, so a sliver along the outline cannot make its neighbour's
//! edge count as shared.

use std::collections::HashMap;

use crate::mesh::{FAR_EDGE, MeshVertex, length, twice_area};

/// SVG's default miter limit: a corner's offset is no longer than
/// this many half pixels, so a spike's tip is bevelled rather than
/// drawn far out.
const MITER_LIMIT: f32 = 4.0;

/// The triangles of `indices` over `vertices`, three indices each.
///
/// # Panics
/// When `indices` is not a whole number of triangles: lyon emits
/// triples, so a remainder is a producer bug.
#[must_use]
pub fn triangles_with_edges(vertices: &[[f32; 2]], indices: &[u32]) -> Vec<[MeshVertex; 3]> {
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
    let uses = edge_uses(&corners);
    let normals = vertex_normals(vertices, &corners, &uses);
    corners
        .iter()
        .map(|corner| {
            let outline = [1, 2, 0].map(|k| uses[&edge_key(corner[k], corner[(k + 1) % 3])] == 1);
            triangle(
                positions(vertices, *corner),
                corner.map(|i| normals[i as usize]),
                outline,
            )
        })
        .collect()
}

fn positions(vertices: &[[f32; 2]], corner: [u32; 3]) -> [[f32; 2]; 3] {
    corner.map(|i| vertices[i as usize])
}

/// How many triangles use each undirected edge; two at most for a
/// mesh this code understands.
fn edge_uses(corners: &[[u32; 3]]) -> HashMap<(u32, u32), u32> {
    let mut uses = HashMap::with_capacity(corners.len() * 3);
    for corner in corners {
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            let count = uses.entry(edge_key(corner[a], corner[b])).or_insert(0);
            *count += 1;
            debug_assert!(
                *count <= 2,
                "edge {:?} is used by {count} triangles; the producer overlaps itself",
                edge_key(corner[a], corner[b])
            );
        }
    }
    uses
}

fn edge_key(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

/// Each vertex's offset direction: the miter offset of its outline
/// edges, clamped at [`MITER_LIMIT`]; zero for a vertex inside; the
/// normalised sum where more than two outline edges meet (a
/// self-touching outline, approximate there).
fn vertex_normals(
    vertices: &[[f32; 2]],
    corners: &[[u32; 3]],
    uses: &HashMap<(u32, u32), u32>,
) -> Vec<[f32; 2]> {
    let mut incident: Vec<Vec<[f32; 2]>> = vec![Vec::new(); vertices.len()];
    for corner in corners {
        let points = positions(vertices, *corner);
        for edge in 0..3 {
            let (b, c) = (corner[(edge + 1) % 3], corner[(edge + 2) % 3]);
            if uses[&edge_key(b, c)] == 1 {
                let n = outward_normal(points, edge);
                incident[b as usize].push(n);
                incident[c as usize].push(n);
            }
        }
    }
    incident.iter().map(|edges| miter_offset(edges)).collect()
}

fn miter_offset(normals: &[[f32; 2]]) -> [f32; 2] {
    match normals {
        [] => [0.0, 0.0],
        [n] => *n,
        [n1, n2] => {
            let dot = n1[0] * n2[0] + n1[1] * n2[1];
            let sum = [n1[0] + n2[0], n1[1] + n2[1]];
            let scale = 1.0 / (1.0 + dot);
            clamp_length([sum[0] * scale, sum[1] * scale])
        }
        many => {
            let sum = many
                .iter()
                .fold([0.0, 0.0], |s, n| [s[0] + n[0], s[1] + n[1]]);
            let len = (sum[0] * sum[0] + sum[1] * sum[1])
                .sqrt()
                .max(f32::MIN_POSITIVE);
            [sum[0] / len, sum[1] / len]
        }
    }
}

fn clamp_length(v: [f32; 2]) -> [f32; 2] {
    let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if len.is_finite() && len <= MITER_LIMIT {
        return v;
    }
    let len = len.max(f32::MIN_POSITIVE);
    [v[0] / len * MITER_LIMIT, v[1] / len * MITER_LIMIT]
}

/// The unit normal of edge `edge` pointing away from corner `edge`.
fn outward_normal(points: [[f32; 2]; 3], edge: usize) -> [f32; 2] {
    let (a, b, c) = (points[edge], points[(edge + 1) % 3], points[(edge + 2) % 3]);
    let (dx, dy) = (c[0] - b[0], c[1] - b[1]);
    let len = length(b, c);
    let n = [dy / len, -dx / len];
    let towards_a = (a[0] - b[0]) * n[0] + (a[1] - b[1]) * n[1];
    if towards_a > 0.0 { [-n[0], -n[1]] } else { n }
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
mod tests;

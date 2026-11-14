//! Each triangle corner's normal for the half-pixel extrusion: the
//! miter offset of the two outline edges that bound the corner's
//! wedge, found by walking round its vertex through the triangles
//! that share its edges. Taken per corner rather than per vertex, so
//! where an outline touches itself at a vertex (a bowtie, the points of
//! an even-odd star) each wedge moves out of its own side.

use crate::mesh::outward_normal;
use crate::mesh_edges::{EdgeTriangles, edge_key};

/// SVG's default miter limit: a corner's offset is no longer than
/// this many half pixels, so a spike's tip is bevelled rather than
/// drawn far out.
pub(crate) const MITER_LIMIT: f32 = 4.0;

/// The normal of every corner of every triangle in `corners`: zero
/// for a corner whose vertex the triangles surround.
///
/// # Errors
/// Two outline edges meeting head-on at a corner (the end of a cut of
/// no width into the shape), where the miter has no direction; names
/// the vertex.
pub(crate) fn corner_normals(
    vertices: &[[f32; 2]],
    corners: &[[u32; 3]],
    edges: &EdgeTriangles,
) -> Result<Vec<[[f32; 2]; 3]>, String> {
    let fans = Fans {
        vertices,
        corners,
        edges,
    };
    (0..corners.len()).map(|t| fans.triangle(t)).collect()
}

struct Fans<'a> {
    vertices: &'a [[f32; 2]],
    corners: &'a [[u32; 3]],
    edges: &'a EdgeTriangles,
}

impl Fans<'_> {
    fn triangle(&self, t: usize) -> Result<[[f32; 2]; 3], String> {
        let c = self.corners[t];
        let mut normals = [[0.0; 2]; 3];
        for (k, normal) in normals.iter_mut().enumerate() {
            *normal = self.corner(t, c[k], [c[(k + 1) % 3], c[(k + 2) % 3]])?;
        }
        Ok(normals)
    }

    /// Corner `v` of triangle `t`, whose two edges there run to `sides`.
    fn corner(&self, t: usize, v: u32, sides: [u32; 2]) -> Result<[f32; 2], String> {
        match sides.map(|other| self.wedge_bound(t, v, other)) {
            [Some(n1), Some(n2)] => miter_offset(n1, n2).ok_or_else(|| {
                format!(
                    "outline edges meet head-on at {:?}, where a cut of no width into the shape ends, with no direction to move out in; close the cut or give it width",
                    self.vertices[v as usize]
                )
            }),
            [None, None] => Ok([0.0, 0.0]),
            _ => panic!(
                "the triangles round vertex {v} close on one side and not the other; the producer's mesh is not a surface"
            ),
        }
    }

    /// From triangle `start` across its edge `v`-`other` and on round
    /// `v`, the outward normal of the first outline edge; `None` when
    /// the walk comes back to `start`, the vertex inside.
    fn wedge_bound(&self, start: usize, v: u32, other: u32) -> Option<[f32; 2]> {
        let (mut t, mut other) = (start, other);
        for _ in 0..=self.corners.len() {
            let users = &self.edges[&edge_key(v, other)];
            if users.len() == 1 {
                return Some(self.outward(t, v, other));
            }
            t = if users[0] == t { users[1] } else { users[0] };
            if t == start {
                return None;
            }
            other = self.third(t, v, other);
        }
        panic!("the walk round vertex {v} does not end; an edge there has more than two triangles")
    }

    fn third(&self, t: usize, v: u32, other: u32) -> u32 {
        let corner = self.corners[t];
        corner[opposite(corner, v, other)]
    }

    /// The outward normal of edge `v`-`other` of triangle `t`.
    fn outward(&self, t: usize, v: u32, other: u32) -> [f32; 2] {
        let corner = self.corners[t];
        let points = corner.map(|i| self.vertices[i as usize]);
        outward_normal(points, opposite(corner, v, other))
    }
}

/// The index of `corner`'s vertex that is neither `v` nor `other`.
fn opposite(corner: [u32; 3], v: u32, other: u32) -> usize {
    (0..3)
        .find(|&k| corner[k] != v && corner[k] != other)
        .expect("a triangle with area has three distinct corners")
}

/// The offset that moves a corner half a pixel out across both of its
/// outline edges (projection 1 on each), clamped at [`MITER_LIMIT`];
/// `None` for edges head-on, where the two normals cancel.
fn miter_offset(n1: [f32; 2], n2: [f32; 2]) -> Option<[f32; 2]> {
    let dot = n1[0] * n2[0] + n1[1] * n2[1];
    let sum = [n1[0] + n2[0], n1[1] + n2[1]];
    if 1.0 + dot <= 0.0 || sum == [0.0, 0.0] {
        return None;
    }
    let scale = 1.0 / (1.0 + dot);
    Some(clamp_length([sum[0] * scale, sum[1] * scale]))
}

fn clamp_length(v: [f32; 2]) -> [f32; 2] {
    let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if len <= MITER_LIMIT {
        return v;
    }
    [v[0] / len * MITER_LIMIT, v[1] / len * MITER_LIMIT]
}

#[cfg(test)]
#[path = "mesh_normals_tests.rs"]
mod tests;

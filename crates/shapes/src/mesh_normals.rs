//! Each triangle corner's wedge (S3a3, one-scene 1.2): the outward
//! normals of the two outline edges that bound the corner's wedge,
//! found by walking round its vertex through the triangles that share
//! its edges, and whether the wedge is convex or reflex (the triangles
//! it holds span less or more than half a turn). Taken per corner
//! rather than per vertex, so where an outline touches itself at a
//! vertex (a bowtie, the points of an even-odd star) each wedge is its
//! own.

use crate::mesh::{MeshCorner, length, outward_normal, twice_area};
use crate::mesh_edges::{EdgeTriangles, edge_key};
use crate::mesh_wedge::{Bound, outline_corner};

/// The corners of every triangle in `corners`: an inside corner where
/// the triangles surround its vertex.
///
/// # Errors
/// A corner `mesh_wedge.rs` refuses: the end of a cut of no width, or
/// triangles that overlap round it; names the vertex.
pub(crate) fn corner_wedges(
    vertices: &[[f32; 2]],
    corners: &[[u32; 3]],
    edges: &EdgeTriangles,
) -> Result<Vec<[MeshCorner; 3]>, String> {
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

/// One side of a corner's walk: the bounding outline edge, if the walk
/// met one, and the angle at the vertex of the triangles passed on the
/// way.
struct Side {
    bound: Option<Bound>,
    angle: f32,
}

impl Fans<'_> {
    fn triangle(&self, t: usize) -> Result<[MeshCorner; 3], String> {
        let c = self.corners[t];
        let mut out = [MeshCorner::INSIDE_AT_ORIGIN; 3];
        for (k, corner) in out.iter_mut().enumerate() {
            *corner = self.corner(t, c[k], [c[(k + 1) % 3], c[(k + 2) % 3]])?;
        }
        Ok(out)
    }

    /// Corner `v` of triangle `t`, whose two edges there run to `sides`.
    fn corner(&self, t: usize, v: u32, sides: [u32; 2]) -> Result<MeshCorner, String> {
        let position = self.vertices[v as usize];
        let [a, b] = sides.map(|other| self.wedge_bound(t, v, other));
        match (a.bound, b.bound) {
            (Some(one), Some(two)) => {
                let angle = self.angle(t, v) + a.angle + b.angle;
                outline_corner(position, [one, two], angle)
            }
            (None, None) => Ok(MeshCorner {
                position,
                ..MeshCorner::INSIDE_AT_ORIGIN
            }),
            _ => panic!(
                "the triangles round vertex {v} close on one side and not the other; the producer's mesh is not a surface"
            ),
        }
    }

    /// From triangle `start` across its edge `v`-`other` and on round
    /// `v`: the first outline edge, `None` when
    /// the walk comes back to `start` (the vertex inside), and the
    /// angle at `v` of the triangles passed.
    fn wedge_bound(&self, start: usize, v: u32, other: u32) -> Side {
        let (mut t, mut other, mut angle) = (start, other, 0.0);
        for _ in 0..=self.corners.len() {
            let users = &self.edges[&edge_key(v, other)];
            if users.len() == 1 {
                let bound = Bound {
                    normal: self.bound_normal(t, v, other),
                    far: self.vertices[other as usize],
                };
                return Side {
                    bound: Some(bound),
                    angle,
                };
            }
            t = if users[0] == t { users[1] } else { users[0] };
            if t == start {
                return Side { bound: None, angle };
            }
            angle += self.angle(t, v);
            other = self.third(t, v, other);
        }
        panic!("the walk round vertex {v} does not end; an edge there has more than two triangles")
    }

    /// Triangle `t`'s angle at its corner `v`.
    fn angle(&self, t: usize, v: u32) -> f32 {
        let corner = self.corners[t];
        let k = (0..3)
            .find(|&k| corner[k] == v)
            .expect("v is a corner of t");
        let at = |i: usize| self.vertices[corner[i % 3] as usize];
        let ray = |i: usize| [at(i)[0] - at(k)[0], at(i)[1] - at(k)[1]];
        let (one, two) = (ray(k + 1), ray(k + 2));
        (one[0] * two[1] - one[1] * two[0])
            .abs()
            .atan2(one[0] * two[0] + one[1] * two[1])
    }

    fn third(&self, t: usize, v: u32, other: u32) -> u32 {
        let corner = self.corners[t];
        corner[opposite(corner, v, other)]
    }

    /// The outward normal of the outline edge `v`-`other` of triangle
    /// `t`. A sliver (`is_sliver`) has no side of its own to lean away
    /// from, so it takes the side across its other edge `v`-`x`: that
    /// neighbour's normal there, away from the neighbour, points on past
    /// the sliver. With no neighbour (the other edge is outline too)
    /// the sliver's own answer stands.
    fn bound_normal(&self, t: usize, v: u32, other: u32) -> [f32; 2] {
        let x = self.third(t, v, other);
        let users = &self.edges[&edge_key(v, x)];
        match users.iter().find(|&&u| u != t) {
            Some(&neighbour) if is_sliver(self.points(t)) => self.outward(neighbour, v, x),
            _ => self.outward(t, v, other),
        }
    }

    fn points(&self, t: usize) -> [[f32; 2]; 3] {
        self.corners[t].map(|i| self.vertices[i as usize])
    }

    /// The outward normal of edge `v`-`other` of triangle `t`.
    fn outward(&self, t: usize, v: u32, other: u32) -> [f32; 2] {
        let corner = self.corners[t];
        let points = corner.map(|i| self.vertices[i as usize]);
        outward_normal(points, opposite(corner, v, other))
    }
}

/// Whether a triangle's width is rounding noise against its length
/// (twice its area under `SLIVER` of its longest edge squared): lyon's
/// flattened arc ends a hair off the point it was told, and the
/// sliver beside the outline then has a coin toss for its third
/// corner's side.
fn is_sliver(points: [[f32; 2]; 3]) -> bool {
    let [a, b, c] = points;
    let longest = length(a, b).max(length(b, c)).max(length(c, a));
    twice_area(points).abs() <= SLIVER * longest * longest
}

/// The most a sliver's width is of its length.
const SLIVER: f32 = 1e-6;

/// The index of `corner`'s vertex that is neither `v` nor `other`.
fn opposite(corner: [u32; 3], v: u32, other: u32) -> usize {
    (0..3)
        .find(|&k| corner[k] != v && corner[k] != other)
        .expect("a triangle with area has three distinct corners")
}

#[cfg(test)]
#[path = "mesh_normals_tests.rs"]
mod tests;

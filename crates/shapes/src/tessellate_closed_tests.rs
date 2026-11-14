//! Closed strokes: lyon's stroker re-creates a subpath's start vertices
//! at the same positions, and the line across the stroke there must
//! still be one shared edge, not two outline edges drawing a seam.

use super::tests::mesh;
use super::*;
use crate::mesh::FAR_EDGE;

/// The midpoint of every outline edge in `mesh`.
fn outline_midpoints(mesh: &Mesh) -> Vec<[f32; 2]> {
    let mut midpoints = Vec::new();
    for triangle in &mesh.triangles {
        for edge in 0..3 {
            if triangle[0].edges[edge] == FAR_EDGE {
                continue;
            }
            let (b, c) = (triangle[(edge + 1) % 3], triangle[(edge + 2) % 3]);
            midpoints.push([0, 1].map(|k| f32::midpoint(b.position[k], c.position[k])));
        }
    }
    midpoints
}

/// Every outline edge of the stroke lies on one of its two sides, at
/// least `half_width - slack` from the centreline `distance` measures.
fn assert_no_edge_across(
    mesh: &Mesh,
    half_width: f32,
    slack: f32,
    distance: impl Fn([f32; 2]) -> f32,
) {
    let midpoints = outline_midpoints(mesh);
    assert!(midpoints.len() > 2, "{midpoints:?}");
    for p in midpoints {
        assert!(
            distance(p) >= half_width - slack,
            "an outline edge's midpoint {p:?} is {} from the centreline: an edge across the stroke",
            distance(p)
        );
    }
}

#[test]
fn a_stroked_closed_square_has_no_outline_edge_across_it_at_its_start() {
    // Given: a square of side 8 from (0, 0), stroked 2 wide
    let mesh = mesh("M0 0h8v8h-8z", Paint::Stroke { width: 2.0 });

    // Then: each outline edge is on the outer or inner side, 1 from the
    // square's sides
    let to_centreline = |[x, y]: [f32; 2]| {
        let outside = [(-x).max(x - 8.0).max(0.0), (-y).max(y - 8.0).max(0.0)];
        if outside != [0.0, 0.0] {
            return outside[0].hypot(outside[1]);
        }
        x.min(8.0 - x).min(y).min(8.0 - y)
    };
    assert_no_edge_across(&mesh, 1.0, 1e-4, to_centreline);
}

#[test]
fn a_stroked_closed_circle_has_no_outline_edge_across_it_at_its_start() {
    // Given: a circle of radius 6 about (8, 8), stroked 1 wide
    let mesh = mesh(
        "M2 8a6 6 0 1 0 12 0a6 6 0 1 0-12 0z",
        Paint::Stroke { width: 1.0 },
    );

    // Then: within the flattening of the circle
    let to_centreline = |[x, y]: [f32; 2]| ((x - 8.0).hypot(y - 8.0) - 6.0).abs();
    assert_no_edge_across(&mesh, 0.5, 0.1, to_centreline);
}

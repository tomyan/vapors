//! Triangles without area are dropped before edges are counted. Along
//! the outline that is right: the sliver's short edges belong to it
//! alone. Between triangles on both sides it is a T-junction inside the
//! shape: the long edge and the short ones would each be used once and
//! draw as outline, a seam across the fill. Nothing shows lyon's fill
//! emitting one, so it is refused, naming the vertex, until a path that
//! does is in hand as a fixture.

use crate::mesh::length;
use crate::mesh_edges::{EdgeTriangles, edge_key};

/// Refuses a dropped sliver whose long edge and a short edge both
/// belong to kept triangles.
///
/// # Errors
/// The T-junction, naming its vertex and the sliver's ends.
pub(crate) fn check_slivers(
    vertices: &[[f32; 2]],
    slivers: &[[u32; 3]],
    edges: &EdgeTriangles,
) -> Result<(), String> {
    for sliver in slivers {
        check_sliver(vertices, *sliver, edges)?;
    }
    Ok(())
}

fn check_sliver(
    vertices: &[[f32; 2]],
    sliver: [u32; 3],
    edges: &EdgeTriangles,
) -> Result<(), String> {
    let [a, b, c] = sliver;
    if a == b || b == c || a == c {
        return Ok(());
    }
    let [end, middle, other_end] = middle_second(vertices, sliver);
    let kept = |p: u32, q: u32| edges.contains_key(&edge_key(p, q));
    if !(kept(end, other_end) && (kept(end, middle) || kept(middle, other_end))) {
        return Ok(());
    }
    let at = |i: u32| vertices[i as usize];
    Err(format!(
        "triangles meet at a T-junction at {:?} inside the shape, across a triangle of no area from {:?} to {:?}; its edges would draw as outline, and the tessellator does not resolve this yet, so report the path as a fixture",
        at(middle),
        at(end),
        at(other_end)
    ))
}

/// `sliver` rotated so the vertex between the other two is second: the
/// one opposite the longest edge.
fn middle_second(vertices: &[[f32; 2]], sliver: [u32; 3]) -> [u32; 3] {
    let at = |i: u32| vertices[i as usize];
    let opposite = |k: usize| length(at(sliver[(k + 1) % 3]), at(sliver[(k + 2) % 3]));
    let middle = (0..3)
        .max_by(|&j, &k| opposite(j).total_cmp(&opposite(k)))
        .unwrap_or(0);
    [0, 1, 2].map(|k| sliver[(middle + 2 + k) % 3])
}

#[cfg(test)]
#[path = "mesh_slivers_tests.rs"]
mod tests;

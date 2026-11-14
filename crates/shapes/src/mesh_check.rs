//! The check that a mesh's numbers are ones a corner can have
//! (`mesh.rs` says the rule): the fragment shader takes coverage from
//! them after the transform, so a wrong one draws a seam or a hard edge
//! rather than failing.

use crate::mesh::{
    CONVEX, INSIDE, MeshCorner, MeshTriangle, REFLEX, SLACK, dot, head_on, length, minus,
    twice_area,
};

/// Why `triangles` cannot be drawn, if they cannot: no triangle; a
/// value that is not finite; a triangle without area; a corner whose
/// kind is none of the three; an inside corner with normals or reach;
/// an outline corner whose normals are not unit length, whose reach is
/// zero, whose reflex normals meet head-on, or whose wedge does not
/// hold its triangle.
///
/// # Errors
/// The reason, naming the triangle and the corner.
pub fn check_mesh(triangles: &[MeshTriangle]) -> Result<(), String> {
    if triangles.is_empty() {
        return Err("a mesh with no triangles draws nothing".to_owned());
    }
    for (t, triangle) in triangles.iter().enumerate() {
        check_triangle(triangle).map_err(|reason| format!("triangle {t} {reason}"))?;
    }
    Ok(())
}

fn check_triangle(triangle: &MeshTriangle) -> Result<(), String> {
    for (c, corner) in triangle.corners.iter().enumerate() {
        check_finite(c, corner)?;
    }
    let points = triangle.corners.map(|corner| corner.position);
    if twice_area(points) == 0.0 {
        return Err(format!("has no area: corners {points:?}"));
    }
    for (c, corner) in triangle.corners.iter().enumerate() {
        check_corner(corner, points).map_err(|reason| format!("corner {c} {reason}"))?;
    }
    Ok(())
}

fn check_finite(c: usize, corner: &MeshCorner) -> Result<(), String> {
    finite(c, "position", &corner.position)?;
    finite(c, "normals", corner.normals.as_flattened())?;
    finite(c, "kind", &[corner.kind])?;
    finite(c, "reaches", &corner.reaches)
}

fn finite(c: usize, field: &str, values: &[f32]) -> Result<(), String> {
    if values.iter().all(|v| v.is_finite()) {
        return Ok(());
    }
    Err(format!("corner {c} {field} {values:?} is not finite"))
}

#[allow(clippy::float_cmp)] // exact: the kinds are written as these values
fn check_corner(corner: &MeshCorner, points: [[f32; 2]; 3]) -> Result<(), String> {
    if corner.kind == INSIDE {
        return check_inside(corner);
    }
    if corner.kind != CONVEX && corner.kind != REFLEX {
        return Err(format!(
            "has kind {}, not convex (1), reflex (-1) or inside (0)",
            corner.kind
        ));
    }
    check_normals(corner)?;
    check_reaches(corner.reaches)?;
    check_wedge_holds(corner, points)
}

#[allow(clippy::float_cmp)] // exact: an inside corner is written with zeros
fn check_inside(corner: &MeshCorner) -> Result<(), String> {
    if corner.normals != [[0.0; 2]; 2] || corner.reaches != [0.0; 2] {
        return Err(format!(
            "is inside but has normals {:?} and reaches {:?}; an inside corner has neither",
            corner.normals, corner.reaches
        ));
    }
    Ok(())
}

#[allow(clippy::float_cmp)] // exact: the kinds are written as these values
fn check_normals(corner: &MeshCorner) -> Result<(), String> {
    for normal in corner.normals {
        let len = length(normal, [0.0, 0.0]);
        if (len - 1.0).abs() > SLACK {
            return Err(format!(
                "has normal {normal:?}, of length {len}, not a unit normal"
            ));
        }
    }
    if corner.kind == REFLEX && head_on(corner.normals) {
        return Err(format!(
            "is reflex with normals {:?} that meet head-on: the end of a cut of no width, whose wedge has no outside",
            corner.normals
        ));
    }
    Ok(())
}

fn check_reaches(reaches: [f32; 2]) -> Result<(), String> {
    if reaches.contains(&0.0) {
        return Err(format!(
            "has reaches {reaches:?}; an outline edge runs some way from its corner"
        ));
    }
    Ok(())
}

/// The corner's wedge holds the triangle: for a convex corner, each of
/// the triangle's corners is inside both lines; for a reflex one, each
/// is inside either line and the triangle's angle at the corner does
/// not hold the outward bisector, so no part of it lies outside both.
#[allow(clippy::float_cmp)] // exact: the kinds are written as these values
fn check_wedge_holds(corner: &MeshCorner, points: [[f32; 2]; 3]) -> Result<(), String> {
    let size = points
        .iter()
        .map(|p| length(*p, corner.position))
        .fold(1.0, f32::max);
    for p in points {
        let out = corner.normals.map(|m| dot(m, minus(p, corner.position)));
        let worst = if corner.kind == CONVEX {
            out[0].max(out[1])
        } else {
            out[0].min(out[1])
        };
        if worst > SLACK * size {
            return Err(format!(
                "has a wedge whose normals {:?} put the triangle's corner {p:?} outside it",
                corner.normals
            ));
        }
    }
    if corner.kind == REFLEX && holds_bisector(corner, points) {
        return Err(format!(
            "is reflex with normals {:?}, and the triangle's angle at it reaches outside both lines",
            corner.normals
        ));
    }
    Ok(())
}

/// Whether the triangle's angle at `corner` holds the direction out of
/// both its lines, `n0 + n1`.
fn holds_bisector(corner: &MeshCorner, points: [[f32; 2]; 3]) -> bool {
    let [n0, n1] = corner.normals;
    let out = [n0[0] + n1[0], n0[1] + n1[1]];
    let rays: Vec<[f32; 2]> = points
        .iter()
        .map(|p| minus(*p, corner.position))
        .filter(|ray| length(*ray, [0.0, 0.0]) > 0.0)
        .collect();
    let cross = |a: [f32; 2], b: [f32; 2]| a[0] * b[1] - a[1] * b[0];
    let (u, w) = (rays[0], rays[1]);
    let turn = cross(u, w);
    turn * cross(u, out) > 0.0 && turn * cross(out, w) > 0.0
}

#[cfg(test)]
#[path = "mesh_check_tests.rs"]
mod tests;

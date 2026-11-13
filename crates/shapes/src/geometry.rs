//! A shape's geometry in viewBox units, and whether it can be drawn.
//! The compiler classifies each literal shape to the cheapest exact
//! primitive (docs/plans/one-scene.md 1.2); the table check refuses a
//! geometry no shader could draw rather than letting it reach one.

/// A shape's geometry, in viewBox units: the cheapest exact primitive
/// the compiler classified it to (one-scene.md 1.2).
#[derive(Debug, PartialEq)]
pub enum Geometry {
    /// An axis-aligned rectangle, rounded when `radius` is not zero,
    /// drawn by the box shader's closed-form distance
    /// (`crates/renderer/src/quads.wgsl`).
    Rect {
        /// Its top-left corner.
        origin: [f32; 2],
        /// Width and height, both above zero.
        size: [f32; 2],
        /// Corner radii `rx`, `ry`: zero or more, at most half the
        /// width and the height (SVG clamps them there; the emitter
        /// clamps before the table), and equal until the shader draws
        /// elliptical corners (S3a).
        radius: [f32; 2],
    },
    /// An ellipse; a circle when the radii are equal.
    Ellipse {
        /// Its centre.
        centre: [f32; 2],
        /// Its radii, both above zero.
        radii: [f32; 2],
    },
    /// Triangles from the tessellator, for every other shape. Winding
    /// is not checked, so the mesh pipeline must not cull faces.
    Mesh {
        /// The vertices the indices name.
        vertices: &'static [MeshVertex],
        /// Three indices per triangle, each below the vertex count.
        indices: &'static [u32],
    },
}

/// A vertex of a tessellated shape: 12 bytes, as docs/plans/svg-support.md
/// 3.1 costs it, in field order, so the table's vertex slices upload
/// as they are.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshVertex {
    /// Its position in viewBox units.
    pub position: [f32; 2],
    /// Its distance to the shape's outline, from which the fragment
    /// shader takes coverage after the transform (one-scene.md 1.2).
    pub outline_distance: f32,
}

const _: () = assert!(size_of::<MeshVertex>() == 12);

/// Why `geometry` cannot be drawn, if it cannot: a value that is not a
/// finite number, a size or radius that is not above zero, a corner
/// radius that is negative, over half the rect's side, or not equal to
/// the other, or a mesh whose indices do not name its vertices as
/// triangles.
///
/// # Errors
/// The reason, without the crate and site [`ShapeTable::check`] adds.
pub fn check_geometry(geometry: &Geometry) -> Result<(), String> {
    match geometry {
        Geometry::Rect {
            origin,
            size,
            radius,
        } => {
            finite("origin", *origin)?;
            positive("size", *size)?;
            check_corners(*size, *radius)
        }
        Geometry::Ellipse { centre, radii } => {
            finite("centre", *centre)?;
            positive("radii", *radii)
        }
        Geometry::Mesh { vertices, indices } => check_mesh(vertices, indices),
    }
}

/// The corner radii the box shader draws: finite, not negative, within
/// half the side (`rounded_distance` cuts each corner in its own r x r
/// square and clamps nothing), and circular: the emitter writes equal
/// radii, so the comparison is exact, not within a margin.
#[allow(clippy::float_cmp)]
fn check_corners(size: [f32; 2], radius: [f32; 2]) -> Result<(), String> {
    finite("radius", radius)?;
    if radius.iter().any(|r| *r < 0.0) {
        return Err(format!("a corner radius of {radius:?} is negative"));
    }
    if radius[0] > size[0] / 2.0 || radius[1] > size[1] / 2.0 {
        return Err(format!(
            "a corner radius of {radius:?} is over half the size {size:?}; the emitter clamps rx to width / 2 and ry to height / 2, as SVG does"
        ));
    }
    if radius[0] != radius[1] {
        return Err(format!(
            "corner radii {radius:?} differ, and the box shader draws circular corners; elliptical corners come with S3a"
        ));
    }
    Ok(())
}

fn finite(name: &str, values: [f32; 2]) -> Result<(), String> {
    if values.iter().all(|v| v.is_finite()) {
        return Ok(());
    }
    Err(format!("{name} {values:?} is not a finite number"))
}

fn positive(name: &str, values: [f32; 2]) -> Result<(), String> {
    finite(name, values)?;
    if values.iter().all(|v| *v > 0.0) {
        return Ok(());
    }
    Err(format!("{name} {values:?} is not above zero"))
}

/// Triangles over finite vertices. A degenerate triangle (repeated or
/// collinear vertices) passes: it covers nothing and draws nothing, as
/// a tessellator may emit at a cusp.
fn check_mesh(vertices: &[MeshVertex], indices: &[u32]) -> Result<(), String> {
    if vertices.is_empty() || indices.is_empty() {
        return Err(format!(
            "{} vertices and {} indices; a mesh names at least one triangle",
            vertices.len(),
            indices.len()
        ));
    }
    if !indices.len().is_multiple_of(3) {
        return Err(format!(
            "{} indices is not a whole number of triangles",
            indices.len()
        ));
    }
    if let Some((at, index)) = indices
        .iter()
        .enumerate()
        .find(|(_, i)| **i as usize >= vertices.len())
    {
        return Err(format!(
            "index {index} at position {at} names no vertex; the mesh has {}",
            vertices.len()
        ));
    }
    match vertices.iter().position(|v| !is_finite(v)) {
        Some(at) => Err(format!("vertex {at} {:?} is not finite", vertices[at])),
        None => Ok(()),
    }
}

fn is_finite(vertex: &MeshVertex) -> bool {
    vertex.position.iter().all(|p| p.is_finite()) && vertex.outline_distance.is_finite()
}

#[cfg(test)]
#[path = "geometry_tests.rs"]
mod tests;

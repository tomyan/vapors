//! A shape's geometry in viewBox units, and whether it can be drawn.
//! The compiler classifies each literal shape to the cheapest exact
//! primitive (docs/plans/one-scene.md 1.2); the table check refuses a
//! geometry no shader could draw rather than letting it reach one.

/// A shape's geometry, in viewBox units: the cheapest exact primitive
/// the compiler classified it to (one-scene.md 1.2).
#[derive(Debug, PartialEq)]
pub enum Geometry {
    /// An axis-aligned rectangle, rounded when `radius` is not zero.
    Rect {
        /// Its top-left corner.
        origin: [f32; 2],
        /// Width and height, both above zero.
        size: [f32; 2],
        /// Corner radii `rx`, `ry`, zero or more.
        radius: [f32; 2],
    },
    /// An ellipse; a circle when the radii are equal.
    Ellipse {
        /// Its centre.
        centre: [f32; 2],
        /// Its radii, both above zero.
        radii: [f32; 2],
    },
    /// Triangles from the tessellator, for every other shape.
    Mesh {
        /// The vertices the indices name.
        vertices: &'static [MeshVertex],
        /// Three indices per triangle, each below the vertex count.
        indices: &'static [u32],
    },
}

/// A vertex of a tessellated shape: 12 bytes, as docs/plans/svg-support.md
/// 3.1 costs it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshVertex {
    /// Its position in viewBox units.
    pub position: [f32; 2],
    /// Its distance to the shape's outline, from which the fragment
    /// shader takes coverage after the transform (one-scene.md 1.2).
    pub outline_distance: f32,
}

/// Why `geometry` cannot be drawn, if it cannot: a value that is not a
/// finite number, a size or radius that is not above zero, a negative
/// corner radius, or a mesh whose indices do not name its vertices as
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
            finite("radius", *radius)?;
            if radius.iter().any(|r| *r < 0.0) {
                return Err(format!("a corner radius of {radius:?} is negative"));
            }
            Ok(())
        }
        Geometry::Ellipse { centre, radii } => {
            finite("centre", *centre)?;
            positive("radii", *radii)
        }
        Geometry::Mesh { vertices, indices } => check_mesh(vertices, indices),
    }
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

fn check_mesh(vertices: &[MeshVertex], indices: &[u32]) -> Result<(), String> {
    if vertices.is_empty() || indices.is_empty() {
        return Err("a mesh with no vertices or no indices draws nothing".to_owned());
    }
    if !indices.len().is_multiple_of(3) {
        return Err(format!(
            "{} indices is not a whole number of triangles",
            indices.len()
        ));
    }
    if let Some(index) = indices.iter().find(|i| **i as usize >= vertices.len()) {
        return Err(format!(
            "index {index} names no vertex; the mesh has {}",
            vertices.len()
        ));
    }
    let bad = vertices
        .iter()
        .find(|v| !(v.position.iter().all(|p| p.is_finite()) && v.outline_distance.is_finite()));
    match bad {
        Some(vertex) => Err(format!("vertex {vertex:?} is not finite")),
        None => Ok(()),
    }
}

#[cfg(test)]
#[path = "geometry_tests.rs"]
mod tests;

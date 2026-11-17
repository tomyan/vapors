//! Solid scenery and car meshes for the Track renderer: the triangles of
//! `world_geometry.rs`, each vertex its position, its part's colour times
//! the face's light, and whether it moves with the car.
use crate::world_geometry::{self, Part, Sink};

#[derive(Default)]
pub(super) struct Mesh {
    pub vertices: Vec<f32>,
}

impl Sink for Mesh {
    fn triangle(&mut self, part: Part, points: [[f32; 3]; 3], light: f32) {
        let colour = part.colour().map(|c| c * light);
        for point in points {
            self.vertices.extend(point);
            self.vertices.extend(colour);
            self.vertices.push(if part.on_car() { 1.0 } else { 0.0 });
        }
    }
}

pub(super) fn scenery() -> Mesh {
    let mut mesh = Mesh::default();
    world_geometry::scenery(&mut mesh);
    mesh
}

pub(super) fn car() -> Mesh {
    let mut mesh = Mesh::default();
    world_geometry::car(&mut mesh);
    mesh
}

#[cfg(test)]
#[path = "mesh_tests.rs"]
mod tests;

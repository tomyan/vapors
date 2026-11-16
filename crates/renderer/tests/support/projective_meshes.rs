//! Drawing mesh items on a plane in a scene pass
//! (`projective.rs`'s target and camera), white over black, and reading
//! each pixel's coverage back.

// each test crate including this uses a part of it
#![allow(dead_code)]

use glam::Mat4;
use vapors_renderer::{MeshShape, ProjectiveMeshes, WorkCounter};
use vapors_scene::Color;

use crate::page::Drawn;
use crate::scene::{buffer, reds, scene_pixels};

/// Each pixel's coverage of `meshes`, drawn in order on the plane
/// `camera.1` places, seen by `camera.0`, on a `side` square target.
pub fn mesh_coverage_3d(side: u32, meshes: &[Drawn], camera: (Mat4, Mat4)) -> Vec<f32> {
    let black = Color::from_rgba8(0, 0, 0, 255);
    let bytes: Vec<(Vec<u8>, Vec<u8>)> = meshes.iter().map(mesh_bytes).collect();
    reds(&scene_pixels(
        side,
        camera,
        black,
        |gpu, layouts, formats, pass, bound| {
            let pipeline = ProjectiveMeshes::new(&gpu.device, layouts, formats);
            let work = WorkCounter::default();
            for (item, triangles) in &bytes {
                let item = buffer(gpu, wgpu::BufferUsages::VERTEX, item);
                let triangles = buffer(gpu, wgpu::BufferUsages::VERTEX, triangles);
                pipeline.record(pass, bound, (item.slice(..), triangles.slice(..)), &work);
            }
        },
    ))
}

fn mesh_bytes((triangles, transform, colour): &Drawn) -> (Vec<u8>, Vec<u8>) {
    let item = MeshShape::new(*transform, *colour)
        .expect("drawable")
        .expect("has area");
    (
        ProjectiveMeshes::item_bytes(&item),
        ProjectiveMeshes::triangle_bytes(triangles),
    )
}

/// Each pixel's coverage of `mesh` (its own triangles, unmoved, in shape
/// units) under `affine` on the plane `camera` sees, drawn on the CPU
/// (`crate::scene::cpu_coverage_8x`): no code shared with the shader.
pub fn cpu_mesh_coverage(
    side: u32,
    mesh: &[vapors_shapes::MeshTriangle],
    affine: [f32; 6],
    camera: (Mat4, Mat4),
) -> Vec<f32> {
    let inside = |p: [f32; 2]| {
        mesh.iter()
            .any(|t| crate::page::contains(t.corners.map(|c| c.position), p))
    };
    crate::scene::cpu_coverage_8x(side, &inside, affine, camera)
}

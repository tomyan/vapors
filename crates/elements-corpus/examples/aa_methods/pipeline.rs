//! The mesh pipeline (one-scene 1.2): each icon tessellated and drawn
//! with its outline moved half a device pixel out and coverage from its
//! triangles' corner wedges, the exact-corner method S3b chose (S3a3).

use vapors_gpu_harness::Gpu;
use vapors_renderer::{MeshEntries, MeshPipeline, MeshShape};
use vapors_scene::{Color, Size};
use vapors_shapes::{IconPath, MeshTriangle, Paint, tessellate};

use crate::cases::{CELL, Icon, Placement, transform};
use crate::offscreen::drawn_on;

/// `icon` stroked and tessellated within `tolerance_px` device pixels
/// at `scale` device pixels per unit.
pub fn mesh(icon: &Icon, tolerance_px: f32, scale: f32) -> Vec<MeshTriangle> {
    let path = IconPath::parse(icon.path).expect("touche's icons parse");
    let paint = if icon.stroke > 0.0 {
        Paint::Stroke { width: icon.stroke }
    } else {
        Paint::Fill(vapors_shapes::FillRule::NonZero)
    };
    tessellate(icon.name, &path, paint, tolerance_px / scale)
        .expect("touche's icons tessellate")
        .triangles
}

/// The page's size in device pixels for `count` cells under `placement`.
pub fn size(count: usize, placement: &Placement) -> Size {
    let count = u32::try_from(count).expect("a few cells");
    Size {
        width: CELL * count * placement.scale,
        height: CELL * placement.scale,
    }
}

/// `icons` drawn by the mesh pipeline in `ink` under `placement`.
pub fn drawn(
    gpu: &Gpu,
    icons: &[&Icon],
    placement: &Placement,
    ink: [u8; 4],
    tolerance_px: f32,
) -> Vec<u8> {
    let meshes: Vec<_> = icons
        .iter()
        .map(|icon| mesh(icon, tolerance_px, scale(placement)))
        .collect();
    let colour = Color::from_rgba8(ink[0], ink[1], ink[2], ink[3]);
    drawn_on(gpu, size(icons.len(), placement), |on| {
        let mut entries = MeshEntries::new(on.device);
        let written: Vec<_> = meshes
            .iter()
            .map(|m| entries.write(on, m).expect("room"))
            .collect();
        let items: Vec<_> = (0u32..)
            .zip(&written)
            .map(|(at, entry)| (shape(transform(placement, at), colour), entry))
            .collect();
        MeshPipeline::new(on.device, crate::offscreen::FORMAT).draw(on, &mut entries, &items);
    })
}

fn shape(transform: [f32; 6], colour: Color) -> MeshShape {
    MeshShape::new(transform, colour)
        .expect("drawable")
        .expect("has area")
}

/// The device pixels per unit the placement draws at, for flattening:
/// the larger of its scale's axes.
pub fn scale(placement: &Placement) -> f32 {
    let [a, b, ..] = placement.linear;
    (a * a + b * b).sqrt() * placement.scale as f32
}

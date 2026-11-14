//! Drawing closed-form shapes white over black on a 32 x 32 target and
//! reading each pixel's coverage back: its red channel, 0 to 1.

// each test crate including this uses a part of it
#![allow(dead_code)]

use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{ClosedFormPipeline, ClosedFormShape, FrameEncoder, Target, clear};
use vapors_scene::{Color, Size};
use vapors_shapes::Geometry;

pub const SIDE: u32 = 32;

/// `geometry` in white under `transform`, stroked `stroke` wide or
/// filled.
pub fn white(geometry: &Geometry, stroke: Option<f32>, transform: [f32; 6]) -> ClosedFormShape {
    let white = Color::from_rgba8(255, 255, 255, 255);
    ClosedFormShape::new(geometry, stroke, transform, white)
        .expect("drawable")
        .expect("has area")
}

/// A rect at `origin` of `size` with corners of `radius`.
pub const fn rect(origin: [f32; 2], size: [f32; 2], radius: f32) -> Geometry {
    Geometry::Rect {
        origin,
        size,
        radius: [radius, radius],
    }
}

/// A circle about `centre` of `radius`.
pub const fn circle(centre: [f32; 2], radius: f32) -> Geometry {
    Geometry::Ellipse {
        centre,
        radii: [radius, radius],
    }
}

/// Scaled by `scale` about the origin.
pub fn scaled(scale: f32) -> [f32; 6] {
    [scale, 0.0, 0.0, scale, 0.0, 0.0]
}

/// Each pixel's coverage, row by row.
pub fn coverage(shapes: &[ClosedFormShape]) -> Vec<f32> {
    let gpu = gpu();
    let target = clear(
        &gpu.device,
        &gpu.queue,
        SIDE,
        SIDE,
        Color::from_rgba8(0, 0, 0, 255),
    );
    let mut pipeline = ClosedFormPipeline::new(&gpu.device, target.format());
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = FrameEncoder::new(&gpu.device);
    let viewport = Size {
        width: SIDE,
        height: SIDE,
    };
    let (device, queue) = (&gpu.device, &gpu.queue);
    let on = Target {
        device,
        queue,
        encoder: &encoder,
        view: &view,
        viewport,
        scissor: None,
    };
    pipeline.draw(&on, shapes);
    encoder.submit(queue).expect("no draw refused");
    let pixels = read_rgba8(device, queue, &target);
    pixels.chunks(4).map(|p| f32::from(p[0]) / 255.0).collect()
}

/// Pixel (`x`, `y`)'s coverage, within one 8-bit step and a little of
/// the rasteriser's rounding of `expected`.
pub fn assert_at(coverage: &[f32], (x, y): (u32, u32), expected: f32, what: &str) {
    let found = coverage[(y * SIDE + x) as usize];
    assert!(
        (found - expected).abs() <= 1.5 / 255.0,
        "{what}: pixel ({x}, {y}) coverage {found}, expected {expected}"
    );
}

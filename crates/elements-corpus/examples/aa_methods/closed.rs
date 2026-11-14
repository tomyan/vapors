//! The closed-form kind's analytic method: each case's rect or ellipse
//! drawn by the closed-form pipeline, coverage from its exact distance.

use vapors_gpu_harness::{Gpu, read_rgba8};
use vapors_renderer::{ClosedFormPipeline, ClosedFormShape, FrameEncoder, Target, clear};
use vapors_scene::Color;

use crate::cases::{ClosedCase, PAGE, Placement, transform};
use crate::pipeline::size;

/// `cases` drawn by the closed-form pipeline in `ink` under `placement`.
pub fn drawn(gpu: &Gpu, cases: &[ClosedCase], placement: &Placement, ink: [u8; 4]) -> Vec<u8> {
    let viewport = size(cases.len(), placement);
    let page = Color::from_rgba8(PAGE[0], PAGE[1], PAGE[2], PAGE[3]);
    let target = clear(
        &gpu.device,
        &gpu.queue,
        viewport.width,
        viewport.height,
        page,
    );
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = FrameEncoder::new(&gpu.device);
    let on = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &encoder,
        view: &view,
        viewport,
        scissor: None,
    };
    let colour = Color::from_rgba8(ink[0], ink[1], ink[2], ink[3]);
    let shapes: Vec<ClosedFormShape> = (0u32..)
        .zip(cases)
        .map(|(at, case)| {
            let stroke = (case.shape.stroke > 0.0).then_some(case.shape.stroke);
            ClosedFormShape::new(&case.geometry, stroke, transform(placement, at), colour)
                .expect("drawable")
                .expect("has area")
        })
        .collect();
    ClosedFormPipeline::new(&gpu.device, target.format()).draw(&on, &shapes);
    encoder.submit(&gpu.queue).expect("no draw refused");
    read_rgba8(&gpu.device, &gpu.queue, &target)
}

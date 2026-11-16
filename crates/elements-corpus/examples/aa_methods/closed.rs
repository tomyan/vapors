//! The closed-form kind's analytic method: each case's rect or ellipse
//! drawn by the closed-form pipeline, coverage from its exact distance.

use vapors_gpu_harness::Gpu;
use vapors_renderer::{ClosedFormPipeline, ClosedFormShape};
use vapors_scene::Color;

use crate::cases::{ClosedCase, Placement, transform};
use crate::offscreen::drawn_on;
use crate::pipeline::size;

/// `cases` drawn by the closed-form pipeline in `ink` under `placement`.
pub fn drawn(gpu: &Gpu, cases: &[ClosedCase], placement: &Placement, ink: [u8; 4]) -> Vec<u8> {
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
    drawn_on(gpu, size(cases.len(), placement), |on| {
        ClosedFormPipeline::new(on.device, crate::offscreen::FORMAT).draw(on, &shapes);
    })
}

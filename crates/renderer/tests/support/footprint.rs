//! Which pixels the scene's closed-form vertex stage gives fragments:
//! the shipped stage (`CLOSED_FORMS_3D_WGSL`) with a fragment that writes
//! white wherever it runs, so the footprint, not the coverage, is read.

// each test crate including this uses a part of it
#![allow(dead_code)]

use glam::Mat4;
use vapors_renderer::projective::{ProjectiveLayouts, SHAPE_BLEND, shape_depth};
use vapors_renderer::{
    CLOSED_FORM_STRIDE, CLOSED_FORMS_3D_WGSL, ClosedFormShape, ProjectiveClosedForms,
    ScenePassFormats, WorkCounter,
};
use vapors_scene::Color;

use crate::scene::{buffer, reds, scene_pixels};

const EVERY_FRAGMENT: &str = "
@fragment
fn fs_footprint(in: Out) -> @location(0) vec4<f32> {
    return vec4(1.0);
}
";

/// 1 where the vertex stage gives `shapes` a fragment, else 0, on a
/// `side` square target under `camera`.
pub fn footprint(side: u32, shapes: &[ClosedFormShape], camera: (Mat4, Mat4)) -> Vec<f32> {
    let bytes = ProjectiveClosedForms::bytes(shapes);
    let black = Color::from_rgba8(0, 0, 0, 255);
    reds(&scene_pixels(
        side,
        camera,
        black,
        |gpu, layouts, formats, pass, bound| {
            let pipeline = pipeline(&gpu.device, layouts, formats);
            let instances = buffer(gpu, wgpu::BufferUsages::VERTEX, &bytes);
            let count = u32::try_from(bytes.len() as u64 / CLOSED_FORM_STRIDE).expect("a test run");
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, bound.camera, &[]);
            pass.set_bind_group(1, bound.node, &[bound.offset]);
            pass.set_vertex_buffer(0, instances.slice(..));
            WorkCounter::default().draw(pass, 0..6, 0..count);
        },
    ))
}

fn pipeline(
    device: &wgpu::Device,
    layouts: &ProjectiveLayouts,
    formats: ScenePassFormats,
) -> wgpu::RenderPipeline {
    let source = [CLOSED_FORMS_3D_WGSL, EVERY_FRAGMENT].concat();
    let shader = vapors_renderer::pipelines::shader_module(
        device,
        wgpu::ShaderModuleDescriptor {
            label: Some("footprint"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        },
    );
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("footprint"),
        bind_group_layouts: &[Some(&layouts.camera), Some(&layouts.node)],
        immediate_size: 0,
    });
    let attributes = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4];
    let instance = wgpu::VertexBufferLayout {
        array_stride: CLOSED_FORM_STRIDE,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &attributes,
    };
    let targets = [Some(wgpu::ColorTargetState {
        format: formats.colour,
        blend: Some(SHAPE_BLEND),
        write_mask: wgpu::ColorWrites::ALL,
    })];
    vapors_renderer::pipelines::render_pipeline(
        device,
        &wgpu::RenderPipelineDescriptor {
            label: Some("footprint"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(instance)],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_footprint"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &targets,
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(shape_depth(formats.depth, formats.bias)),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        },
    )
}

/// How far device point `p` lies outside the convex quad `corners` (in
/// order round it), device pixels; 0 inside.
pub fn outside_quad(corners: [[f32; 2]; 4], p: [f32; 2]) -> f32 {
    let side =
        |a: [f32; 2], b: [f32; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
    let edges: Vec<_> = (0..4).map(|k| (corners[k], corners[(k + 1) % 4])).collect();
    let sides: Vec<f32> = edges.iter().map(|(a, b)| side(*a, *b)).collect();
    if sides.iter().all(|s| *s >= 0.0) || sides.iter().all(|s| *s <= 0.0) {
        return 0.0;
    }
    edges
        .iter()
        .map(|(a, b)| segment_distance(p, *a, *b))
        .fold(f32::INFINITY, f32::min)
}

fn segment_distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    (a[0] + t * dx - p[0]).hypot(a[1] + t * dy - p[1])
}

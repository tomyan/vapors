//! A `SurfaceRenderer` drawing `gradient.wgsl` over its surface, moved
//! a step along at each render, and counting its renders for the check.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use vapors_app::{SURFACE_FORMAT, SurfaceError, SurfaceFrame, SurfaceRenderer};

/// Renders for the gradient to move through two widths and back.
const CYCLE: u64 = 240;

/// Draws the gradient; its pipeline and shift are made in `mount`.
pub struct Gradient {
    drawing: Option<Drawing>,
    renders: Arc<AtomicU64>,
}

/// The pipeline, and the `shift` uniform it reads.
struct Drawing {
    pipeline: wgpu::RenderPipeline,
    shift: wgpu::Buffer,
    bindings: wgpu::BindGroup,
}

impl Gradient {
    /// A renderer that adds one to `renders` for each render.
    pub fn counting(renders: Arc<AtomicU64>) -> Self {
        Self {
            drawing: None,
            renders,
        }
    }
}

impl SurfaceRenderer for Gradient {
    fn mount(&mut self, device: &wgpu::Device) -> Result<(), SurfaceError> {
        self.drawing = Some(Drawing::new(device));
        Ok(())
    }

    fn render(&mut self, frame: SurfaceFrame<'_>) -> Result<(), SurfaceError> {
        let drawing = self
            .drawing
            .as_ref()
            .ok_or("the gradient rendered before it was mounted")?;
        let renders = self.renders.fetch_add(1, Ordering::SeqCst);
        #[allow(clippy::cast_precision_loss)] // under CYCLE
        let shift = (renders % CYCLE) as f32 / CYCLE as f32;
        frame
            .queue
            .write_buffer(&drawing.shift, 0, &shift.to_ne_bytes());
        let mut pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gradient"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        pass.set_pipeline(&drawing.pipeline);
        pass.set_bind_group(0, &drawing.bindings, &[]);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}

impl Drawing {
    fn new(device: &wgpu::Device) -> Self {
        let pipeline = pipeline(device);
        let shift = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gradient shift"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gradient shift"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shift.as_entire_binding(),
            }],
        });
        Self {
            pipeline,
            shift,
            bindings,
        }
    }
}

fn pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let module =
        vapors_renderer::pipelines::shader_module(device, wgpu::include_wgsl!("gradient.wgsl"));
    vapors_renderer::pipelines::render_pipeline(
        device,
        &wgpu::RenderPipelineDescriptor {
            label: Some("gradient"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(SURFACE_FORMAT.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        },
    )
}

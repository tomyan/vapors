//! A `SurfaceRenderer` drawing `gradient.wgsl` over its surface, and
//! counting its renders for the idle check.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use vapors_app::{SURFACE_FORMAT, SurfaceError, SurfaceFrame, SurfaceRenderer};

/// Draws the gradient; its pipeline is made in `mount`.
pub struct Gradient {
    pipeline: Option<wgpu::RenderPipeline>,
    renders: Arc<AtomicU64>,
}

impl Gradient {
    /// A renderer that adds one to `renders` for each render.
    pub fn counting(renders: Arc<AtomicU64>) -> Self {
        Self {
            pipeline: None,
            renders,
        }
    }
}

impl SurfaceRenderer for Gradient {
    fn mount(&mut self, device: &wgpu::Device) -> Result<(), SurfaceError> {
        self.pipeline = Some(pipeline(device));
        Ok(())
    }

    fn render(&mut self, frame: SurfaceFrame<'_>) -> Result<(), SurfaceError> {
        let pipeline = self
            .pipeline
            .as_ref()
            .ok_or("the gradient rendered before it was mounted")?;
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
        pass.set_pipeline(pipeline);
        pass.draw(0..3, 0..1);
        self.renders.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::include_wgsl!("gradient.wgsl"));
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
    })
}

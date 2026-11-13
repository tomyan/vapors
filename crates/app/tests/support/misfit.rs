//! A renderer whose pipeline targets `Bgra8Unorm`, so drawing with it
//! into the `Rgba8Unorm` backing is a GPU validation error. It draws
//! with it once; later renders record nothing.

use vapors_app::{SurfaceError, SurfaceFrame, SurfaceRenderer};

const SHADER: &str = "
@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }
@fragment fn fs() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }
";

#[derive(Default)]
pub struct Misfit {
    pipeline: Option<wgpu::RenderPipeline>,
    drawn: bool,
}

impl SurfaceRenderer for Misfit {
    fn mount(&mut self, device: &wgpu::Device) -> Result<(), SurfaceError> {
        self.pipeline = Some(pipeline(device, wgpu::TextureFormat::Bgra8Unorm));
        Ok(())
    }

    fn render(&mut self, frame: SurfaceFrame<'_>) -> Result<(), SurfaceError> {
        let pipeline = self.pipeline.as_ref().ok_or("render before mount")?;
        if std::mem::replace(&mut self.drawn, true) {
            return Ok(());
        }
        let mut pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("misfit"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations::default(),
                })],
                ..Default::default()
            });
        pass.set_pipeline(pipeline);
        pass.draw(0..1, 0..1);
        Ok(())
    }
}

fn pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let module = module(device);
    let targets = [Some(format.into())];
    vapors_renderer::pipelines::render_pipeline(
        device,
        &wgpu::RenderPipelineDescriptor {
            label: Some("misfit"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(fragment(&module, &targets)),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        },
    )
}

fn module(device: &wgpu::Device) -> wgpu::ShaderModule {
    vapors_renderer::pipelines::shader_module(
        device,
        wgpu::ShaderModuleDescriptor {
            label: Some("misfit"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        },
    )
}

fn fragment<'a>(
    module: &'a wgpu::ShaderModule,
    targets: &'a [Option<wgpu::ColorTargetState>],
) -> wgpu::FragmentState<'a> {
    wgpu::FragmentState {
        module,
        entry_point: Some("fs"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        targets,
    }
}

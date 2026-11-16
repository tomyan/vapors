//! The measurement's own targets, passes and pipelines, for the
//! methods the renderer does not have (MSAA, the corner prototype), and
//! the renderer's target for the methods it has.

use vapors_gpu_harness::{Gpu, read_rgba8};
use vapors_renderer::pipelines::{render_pipeline, shader_module};
use vapors_renderer::{FrameEncoder, Target, clear};
use vapors_scene::Color;

use crate::cases::PAGE;

/// The page's pixels after `draw` records into a renderer target of
/// `viewport` cleared to the page colour.
pub fn drawn_on(
    gpu: &Gpu,
    viewport: vapors_scene::Size,
    draw: impl FnOnce(&Target<'_>),
) -> Vec<u8> {
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
    draw(&Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &encoder,
        view: &view,
        viewport,
        scissor: None,
    });
    encoder.submit(&gpu.queue).expect("no draw refused");
    read_rgba8(&gpu.device, &gpu.queue, &target)
}

/// The format of every target here, `clear`'s as well.
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// A target of `size` with `samples` samples a pixel; a single-sample
/// one can be read back.
pub fn texture_for(gpu: &Gpu, size: vapors_scene::Size, samples: u32) -> wgpu::Texture {
    let usage = if samples == 1 {
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC
    } else {
        wgpu::TextureUsages::RENDER_ATTACHMENT
    };
    gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("aa_methods"),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage,
        view_formats: &[],
    })
}

/// A pass over `view` cleared to `page`, resolved into `resolve` if any.
pub fn cleared_pass<'e>(
    encoder: &'e mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    resolve: Option<&wgpu::TextureView>,
    page: [u8; 4],
) -> wgpu::RenderPass<'e> {
    let page = page.map(|c| f64::from(c) / 255.0);
    let store = if resolve.is_some() {
        wgpu::StoreOp::Discard
    } else {
        wgpu::StoreOp::Store
    };
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("aa_methods"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: resolve,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: page[0],
                    g: page[1],
                    b: page[2],
                    a: page[3],
                }),
                store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}

/// A pipeline of `source`'s `vs_main` and `fs_main` over one vertex
/// buffer of `attributes`, `stride` bytes a vertex, alpha blended, at
/// `samples` samples.
pub fn pipeline_of(
    device: &wgpu::Device,
    source: &str,
    (attributes, stride): (&[wgpu::VertexAttribute], u64),
    samples: u32,
) -> wgpu::RenderPipeline {
    let shader = shader_module(
        device,
        wgpu::ShaderModuleDescriptor {
            label: Some("aa_methods"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        },
    );
    let buffers = [Some(wgpu::VertexBufferLayout {
        array_stride: stride,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes,
    })];
    let targets = [Some(wgpu::ColorTargetState {
        format: FORMAT,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    })];
    render_pipeline(device, &descriptor(&shader, &buffers, &targets, samples))
}

fn descriptor<'a>(
    shader: &'a wgpu::ShaderModule,
    buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    targets: &'a [Option<wgpu::ColorTargetState>],
    samples: u32,
) -> wgpu::RenderPipelineDescriptor<'a> {
    let options = wgpu::PipelineCompilationOptions::default;
    wgpu::RenderPipelineDescriptor {
        label: Some("aa_methods"),
        layout: None,
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: options(),
            buffers,
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: options(),
            targets,
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: samples,
            ..Default::default()
        },
        multiview_mask: None,
        cache: None,
    }
}

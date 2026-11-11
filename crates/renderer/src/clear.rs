//! The first render pass: a texture filled with one colour.

use vapors_scene::Color;

use crate::Target;

/// Create an `Rgba8Unorm` texture of `width` by `height` and clear it to
/// `color`. The texture can be copied from, so tests can read it back.
#[must_use]
pub fn clear(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    color: Color,
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("clear target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    clear_view(device, queue, &view, color);
    texture
}

/// Clear an existing render target to `color`, in a submit of its own.
pub fn clear_view(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &wgpu::TextureView,
    color: Color,
) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("clear"),
    });
    clear_pass(&mut encoder, view, color);
    queue.submit([encoder.finish()]);
}

/// Record a clear of `target`'s whole view to `color` into its encoder,
/// in the render pass the draws on that view that follow share.
pub fn clear_target(target: &Target<'_>, color: Color) {
    target.encoder.clear(target.view, to_wgpu(color));
}

fn clear_pass(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, color: Color) {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(to_wgpu(color)),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
}

fn to_wgpu(color: Color) -> wgpu::Color {
    wgpu::Color {
        r: f64::from(color.r),
        g: f64::from(color.g),
        b: f64::from(color.b),
        a: f64::from(color.a),
    }
}

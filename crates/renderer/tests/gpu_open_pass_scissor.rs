//! GPU tier: a draw with no scissor is scissored to its target's
//! viewport, not to the texture's first mip level, so a view of a
//! smaller mip level can be drawn into.

#[path = "support/frame.rs"]
mod frame;

use frame::{quad, target};
use vapors_gpu_harness::gpu;
use vapors_renderer::{FrameEncoder, QuadPipeline};

const RED: [u8; 4] = [255, 0, 0, 255];

#[test]
fn gpu_a_draw_into_mip_level_one_with_no_scissor_records_without_a_validation_error() {
    // Given: an 8 by 8 texture with two mip levels, and a view of the 4 by 4 one
    let gpu = gpu();
    let texture = two_level_texture(&gpu.device, 8);
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        base_mip_level: 1,
        mip_level_count: Some(1),
        ..Default::default()
    });
    let mut pipeline = QuadPipeline::new(&gpu.device, texture.format());
    let encoder = FrameEncoder::new(&gpu.device);
    let scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);

    // When
    pipeline.draw(
        &target(&gpu, &encoder, &view, 4),
        &[quad([0.0, 0.0, 4.0, 4.0], RED)],
    );
    encoder.submit(&gpu.queue).expect("no draw refused");

    // Then
    let error = pollster::block_on(scope.pop());
    assert!(error.is_none(), "validation error: {error:?}");
}

fn two_level_texture(device: &wgpu::Device, side: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("two levels"),
        size: wgpu::Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: 1,
        },
        mip_level_count: 2,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

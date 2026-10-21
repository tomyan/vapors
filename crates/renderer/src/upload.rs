//! Getting pixels onto the GPU.

/// Create an `Rgba8Unorm` texture from tightly packed RGBA bytes, usable
/// for sampling and for copying back out.
///
/// # Panics
///
/// If `rgba.len()` is not `width * height * 4`; the message gives both
/// numbers. That is a caller bug, not an input condition.
#[must_use]
pub fn upload_rgba8(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> wgpu::Texture {
    let expected = width as usize * height as usize * 4;
    assert_eq!(
        rgba.len(),
        expected,
        "upload_rgba8: {}x{} needs {expected} bytes, got {}",
        width,
        height,
        rgba.len()
    );
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("uploaded rgba8"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        size,
    );
    texture
}

use super::*;

#[test]
fn the_painter_accepts_exactly_the_formats_the_platform_picks_for_a_surface() {
    // Given: every format a surface offers on the platforms vapors runs on
    let offered = [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureFormat::Bgra8Unorm,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureFormat::Rgb10a2Unorm,
    ];

    // Then: a format the surface would show, the painter draws, and no other
    for format in offered {
        assert_eq!(
            vapors_platform::pick_format(&[format]).is_some(),
            SUPPORTED.contains(&format),
            "{format:?}"
        );
    }
}

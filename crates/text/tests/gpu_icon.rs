//! GPU tier: an icon draws as one quad of its atlas coverage at its
//! content box, tinted by its colour with a plain blend: no text gamma
//! or contrast boost (docs/icons.md, Paint).

use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::clear;
use vapors_scene::{Color, Size};
use vapors_text::{
    AtlasConfig, GlyphAtlas, Icon, IconDraw, IconPath, IconStroke, RenderTarget, TextPipeline,
};

const SIDE: u32 = 24;
const ORIGIN: [i32; 2] = [4, 5];
const BLACK: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};
const WHITE: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};

/// A 1-unit stroke across y = 8 of a 16-unit view box: half coverage in
/// rows 7 and 8, where a contrast boost would show.
fn half_covered_line() -> Icon {
    let path = IconPath::parse("M0 8H16").expect("parses");
    let strokes = vec![IconStroke { path, width: 1.0 }];
    Icon::new([0.0, 0.0, 16.0, 16.0], strokes).expect("a valid icon")
}

fn render(icon: &Icon) -> Vec<u8> {
    let gpu = gpu();
    let mut atlas = GlyphAtlas::new(AtlasConfig { page_size: 64 });
    let target = clear(&gpu.device, &gpu.queue, SIDE, SIDE, WHITE);
    let mut pipeline = TextPipeline::new(&gpu.device, target.format(), &atlas);
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let render_target = RenderTarget {
        device: &gpu.device,
        queue: &gpu.queue,
        view: &view,
        viewport: Size {
            width: SIDE,
            height: SIDE,
        },
        scissor: None,
    };
    let draw = IconDraw {
        icon,
        origin: ORIGIN,
        size: [16, 16],
        colour: BLACK,
    };
    pipeline
        .draw_icons(&render_target, &mut atlas, &[draw])
        .expect("draws");
    read_rgba8(&gpu.device, &gpu.queue, &target)
}

/// White minus the icon's CPU coverage inside its box, white outside.
fn expected_red(icon: &Icon, x: u32, y: u32) -> f32 {
    let coverage = icon.rasterize([16, 16]);
    let (bx, by) = (x.wrapping_sub(4), y.wrapping_sub(5));
    if bx >= 16 || by >= 16 {
        return 255.0;
    }
    255.0 - f32::from(coverage.data[(by * 16 + bx) as usize])
}

#[test]
fn gpu_an_icon_is_its_coverage_at_its_box_blended_without_text_gamma() {
    // Given
    let icon = half_covered_line();

    // When
    let pixels = render(&icon);

    // Then: every pixel within 1/255 of the plain blend
    for y in 0..SIDE {
        for x in 0..SIDE {
            let actual = f32::from(pixels[((y * SIDE + x) * 4) as usize]);
            let expected = expected_red(&icon, x, y);
            assert!(
                (actual - expected).abs() <= 1.0,
                "pixel ({x}, {y}) is {actual}, plain blend gives {expected}"
            );
        }
    }
}

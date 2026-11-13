//! GPU tier: an icon draws as one quad of its atlas coverage at its
//! content box, tinted by its colour with a plain blend: no text gamma
//! or contrast boost (docs/icons.md, Paint).

use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, clear};
use vapors_scene::{Color, Size};
use vapors_shapes::IconPath;
use vapors_text::{
    AtlasConfig, GlyphAtlas, Icon, IconDraw, IconStroke, RenderTarget, TextPipeline,
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

fn render(icon: &Icon, colour: Color) -> Vec<u8> {
    let gpu = gpu();
    let mut atlas = GlyphAtlas::new(AtlasConfig::fixed(64));
    let target = clear(&gpu.device, &gpu.queue, SIDE, SIDE, WHITE);
    let mut pipeline = TextPipeline::new(&gpu.device, target.format(), &atlas);
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = FrameEncoder::new(&gpu.device);
    let render_target = RenderTarget {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &encoder,
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
        colour,
    };
    pipeline
        .draw_icons(&render_target, &mut atlas, &[draw])
        .expect("draws");
    encoder.submit(&gpu.queue).expect("no draw refused");
    read_rgba8(&gpu.device, &gpu.queue, &target)
}

/// The plain blend of `channel` over white at the icon's CPU coverage
/// times the colour's alpha inside its box; white outside.
fn expected(icon: &Icon, x: u32, y: u32, channel: f32, alpha: f32) -> f32 {
    let coverage = icon.rasterize([16, 16]);
    let (bx, by) = (x.wrapping_sub(4), y.wrapping_sub(5));
    if bx >= 16 || by >= 16 {
        return 255.0;
    }
    let ink = f32::from(coverage.data[(by * 16 + bx) as usize]) / 255.0 * alpha;
    255.0 * (channel * ink + 1.0 - ink)
}

/// Every channel of every pixel within 1/255 of the plain blend.
fn assert_plain_blend(icon: &Icon, colour: Color) {
    let pixels = render(icon, colour);
    let channels = [colour.r, colour.g, colour.b];
    for y in 0..SIDE {
        for x in 0..SIDE {
            for (index, channel) in channels.iter().enumerate() {
                let actual = f32::from(pixels[((y * SIDE + x) * 4) as usize + index]);
                let expected = expected(icon, x, y, *channel, colour.a);
                assert!(
                    (actual - expected).abs() <= 1.0,
                    "pixel ({x}, {y}) channel {index} is {actual}, plain blend gives {expected}"
                );
            }
        }
    }
}

#[test]
fn gpu_an_icon_is_its_coverage_at_its_box_blended_without_text_gamma() {
    assert_plain_blend(&half_covered_line(), BLACK);
}

#[test]
fn gpu_a_translucent_coloured_icon_blends_each_channel_at_coverage_times_alpha() {
    // Given: a blue that differs in every channel, at half alpha
    let colour = Color {
        r: 0.2,
        g: 0.6,
        b: 0.9,
        a: 0.5,
    };

    // Then
    assert_plain_blend(&half_covered_line(), colour);
}

#[test]
fn gpu_overlapping_paths_of_a_translucent_colour_compound_as_separate_paints() {
    // Given: two 2-unit paths over the same line, fully covering rows 7 and 8
    let strokes = (0..2)
        .map(|_| IconStroke {
            path: IconPath::parse("M0 8H16").expect("parses"),
            width: 2.0,
        })
        .collect();
    let icon = Icon::new([0.0, 0.0, 16.0, 16.0], strokes).expect("a valid icon");
    let half_black = Color { a: 0.5, ..BLACK };

    // When
    let pixels = render(&icon, half_black);

    // Then: ink 0.5 + 0.5(1 - 0.5) = 0.75 as in Chromium, not one paint's 0.5
    let red = pixels[(((5 + 7) * SIDE + 10) * 4) as usize];
    assert!((63..=65).contains(&red), "{red}");
}

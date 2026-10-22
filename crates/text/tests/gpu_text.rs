//! GPU tier: glyphs land where the layout puts them, quarter-pixel x
//! offsets are real, baselines snap, and the contrast adjustment keeps
//! stems the same width light-on-dark and dark-on-light.

use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::clear;
use vapors_scene::{Color, Size};
use vapors_text::{
    AtlasConfig, BlendParams, Family, FontSystem, GlyphAtlas, Layout, LayoutOptions, RenderTarget,
    Style, TextPipeline,
};

const WIDTH: u32 = 64;
const HEIGHT: u32 = 48;
const DARK: Color = Color {
    r: 23.0 / 255.0,
    g: 27.0 / 255.0,
    b: 31.0 / 255.0,
    a: 1.0,
};
const LIGHT: Color = Color {
    r: 240.0 / 255.0,
    g: 240.0 / 255.0,
    b: 240.0 / 255.0,
    a: 1.0,
};

/// Render `text` in Geist 16 px with its first baseline starting at
/// `baseline` (x, y) in `ink` on `background`.
fn render(text: &str, baseline: [f32; 2], ink: Color, background: Color) -> Vec<u8> {
    let params = BlendParams::default();
    let gpu = gpu();
    let mut system = FontSystem::bundle_only();
    let style = Style {
        color: ink,
        ..Style::new(Family::Geist, 16.0)
    };
    let layout = Layout::new(&mut system, text, &style, &LayoutOptions::default());
    let first_baseline = layout.lines().next().expect("a line").baseline;
    let origin = [baseline[0], baseline[1] - first_baseline];
    let mut atlas = GlyphAtlas::new(AtlasConfig { page_size: 128 });
    let target = clear(&gpu.device, &gpu.queue, WIDTH, HEIGHT, background);
    let mut pipeline = TextPipeline::new(&gpu.device, target.format(), &atlas);
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let viewport = Size {
        width: WIDTH,
        height: HEIGHT,
    };
    let render_target = RenderTarget {
        device: &gpu.device,
        queue: &gpu.queue,
        view: &view,
        viewport,
    };
    pipeline
        .draw(&render_target, &mut atlas, &layout, origin, params)
        .expect("draws");
    read_rgba8(&gpu.device, &gpu.queue, &target)
}

fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let start = ((y * WIDTH + x) * 4) as usize;
    pixels[start..start + 4].try_into().expect("four bytes")
}

/// How far a pixel is from `background` towards `ink`, 0..=1, by red channel.
fn ink_fraction(px: [u8; 4], ink: Color, background: Color) -> f32 {
    let [ink, back] = [ink.to_rgba8()[0], background.to_rgba8()[0]].map(f32::from);
    (f32::from(px[0]) - back) / (ink - back)
}

fn is_background(px: [u8; 4], background: Color) -> bool {
    px == background.to_rgba8()
}

#[test]
fn gpu_a_16px_glyph_inks_its_box_above_the_baseline_and_nothing_outside() {
    // Given: a capital I on the baseline y = 30 from x = 10; Geist's cap height is ~11.4 px
    let pixels = render("I", [10.0, 30.0], LIGHT, DARK);

    // When
    let mut inked = Vec::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if !is_background(pixel(&pixels, x, y), DARK) {
                inked.push((x, y));
            }
        }
    }

    // Then
    assert!(!inked.is_empty(), "nothing was drawn");
    assert!(
        inked
            .iter()
            .all(|&(x, y)| (10..16).contains(&x) && (17..30).contains(&y)),
        "ink outside the glyph box: {inked:?}"
    );
    assert!(
        inked.iter().any(|&(_, y)| y == 29),
        "ink touches the baseline row"
    );
}

#[test]
fn gpu_a_quarter_pixel_x_offset_changes_the_rendering() {
    // Given / When
    let at_zero = render("H", [10.0, 30.0], LIGHT, DARK);
    let at_quarter = render("H", [10.25, 30.0], LIGHT, DARK);

    // Then
    assert_ne!(at_zero, at_quarter);
}

#[test]
fn gpu_a_fractional_baseline_renders_exactly_like_the_snapped_one() {
    // Given / When
    let snapped = render("Hg", [5.0, 30.0], LIGHT, DARK);
    let fractional = render("Hg", [5.0, 30.4], LIGHT, DARK);

    // Then
    assert_eq!(snapped, fractional);
}

#[test]
fn gpu_stem_widths_match_light_on_dark_and_dark_on_light_within_a_pixel() {
    // Given
    let light_on_dark = render("I", [10.0, 30.0], LIGHT, DARK);
    let dark_on_light = render("I", [10.0, 30.0], DARK, LIGHT);

    // When: the widest row's ink, in pixels, for each
    let widest = |pixels: &[u8], ink: Color, background: Color| {
        (0..HEIGHT)
            .map(|y| {
                (0..WIDTH)
                    .map(|x| ink_fraction(pixel(pixels, x, y), ink, background).clamp(0.0, 1.0))
                    .sum::<f32>()
            })
            .fold(0.0f32, f32::max)
    };
    let light_stem = widest(&light_on_dark, LIGHT, DARK);
    let dark_stem = widest(&dark_on_light, DARK, LIGHT);

    // Then
    assert!(
        light_stem > 0.5 && dark_stem > 0.5,
        "{light_stem} {dark_stem}"
    );
    assert!(
        (light_stem - dark_stem).abs() <= 1.0,
        "light-on-dark stem {light_stem} px, dark-on-light {dark_stem} px"
    );
}

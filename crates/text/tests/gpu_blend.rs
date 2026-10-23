//! GPU tier: the shader's gamma and contrast do what docs/text.md says.
//! With gamma 1 and contrast 0 it is a plain linear blend of coverage;
//! the contrast term changes the picture; at the defaults light-on-dark
//! text comes out heavier than dark-on-light, and no more than a pixel.

use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::clear;
use vapors_scene::{Color, Size};
use vapors_text::{
    AtlasConfig, BlendParams, Family, FontSystem, GlyphAtlas, GlyphRequest, Hinting, Layout,
    LayoutOptions, Rasterizer, RenderTarget, Style, TextPipeline,
};

const WIDTH: u32 = 64;
const HEIGHT: u32 = 48;
const BASELINE: [f32; 2] = [10.0, 30.0];
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
const GREY: Color = Color {
    r: 128.0 / 255.0,
    g: 128.0 / 255.0,
    b: 128.0 / 255.0,
    a: 1.0,
};

/// Render `text` in Geist 16 px from the baseline point `BASELINE`.
fn render(text: &str, ink: Color, background: Color, params: BlendParams) -> Vec<u8> {
    let gpu = gpu();
    let mut system = FontSystem::bundle_only();
    let style = Style {
        color: ink,
        ..Style::new(Family::Geist, 16.0)
    };
    let layout = Layout::new(&mut system, text, &style, &LayoutOptions::default());
    let first_baseline = layout.lines().next().expect("a line").baseline;
    let origin = [BASELINE[0], BASELINE[1] - first_baseline];
    let mut atlas = GlyphAtlas::new(AtlasConfig { page_size: 128 });
    let target = clear(&gpu.device, &gpu.queue, WIDTH, HEIGHT, background);
    let mut pipeline = TextPipeline::new(&gpu.device, target.format(), &atlas);
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let render_target = RenderTarget {
        device: &gpu.device,
        queue: &gpu.queue,
        view: &view,
        viewport: Size {
            width: WIDTH,
            height: HEIGHT,
        },
        scissor: None,
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

/// Mean of how far each pixel is from `background` towards `ink`, by red.
fn mean_ink(pixels: &[u8], ink: Color, background: Color) -> f32 {
    let [ink, back] = [ink.to_rgba8()[0], background.to_rgba8()[0]].map(f32::from);
    let total: f32 = (0..HEIGHT)
        .flat_map(|y| (0..WIDTH).map(move |x| (x, y)))
        .map(|(x, y)| (f32::from(pixel(pixels, x, y)[0]) - back) / (ink - back))
        .sum();
    #[allow(clippy::cast_precision_loss)] // small image
    let count = (WIDTH * HEIGHT) as f32;
    total / count
}

#[test]
fn gpu_gamma_one_and_contrast_zero_is_a_plain_linear_blend_of_coverage() {
    // Given: the I rendered with no adjustment, and its CPU coverage
    let params = BlendParams {
        gamma: 1.0,
        contrast: 0.0,
    };
    let pixels = render("I", LIGHT, DARK, params);
    let mut system = FontSystem::bundle_only();
    let layout = Layout::new(
        &mut system,
        "I",
        &Style::new(Family::Geist, 16.0),
        &LayoutOptions::default(),
    );
    let glyph = layout.glyphs().next().expect("one glyph");
    let coverage = Rasterizer::new()
        .rasterize(&GlyphRequest {
            font: glyph.font,
            id: glyph.id,
            size: glyph.size,
            coords: glyph.coords,
            subpixel: 0,
            hinting: Hinting::Light,
            skew: 0.0,
        })
        .expect("rasterises");

    // When / Then: every pixel is background + coverage * (ink - background), within 1/255
    for cy in 0..coverage.height {
        for cx in 0..coverage.width {
            let x = (10 + coverage.left + cx.cast_signed()).cast_unsigned();
            let y = (30 + coverage.top + cy.cast_signed()).cast_unsigned();
            let c = f32::from(coverage.data[(cy * coverage.width + cx) as usize]) / 255.0;
            let expected = 23.0 + c * (240.0 - 23.0);
            let actual = f32::from(pixel(&pixels, x, y)[0]);
            assert!(
                (actual - expected).abs() <= 1.0,
                "({x}, {y}): {actual} vs {expected}"
            );
        }
    }
}

#[test]
fn gpu_the_contrast_term_changes_mid_grey_text() {
    // Given / When
    let without = render(
        "H",
        GREY,
        DARK,
        BlendParams {
            gamma: 1.2,
            contrast: 0.0,
        },
    );
    let with = render(
        "H",
        GREY,
        DARK,
        BlendParams {
            gamma: 1.2,
            contrast: 0.2,
        },
    );

    // Then
    assert_ne!(without, with);
}

#[test]
fn gpu_light_on_dark_is_heavier_than_dark_on_light_by_a_few_percent_at_the_defaults() {
    // Given
    let light_on_dark = render("Hamburg", LIGHT, DARK, BlendParams::default());
    let dark_on_light = render("Hamburg", DARK, LIGHT, BlendParams::default());

    // When
    let light = mean_ink(&light_on_dark, LIGHT, DARK);
    let dark = mean_ink(&dark_on_light, DARK, LIGHT);

    // Then: the adjustment lifts light text (measured 5.7% at 1.2 / 0.2) but not past a tenth
    assert!(
        light > dark * 1.02,
        "light-on-dark {light} vs dark-on-light {dark}"
    );
    assert!(
        light < dark * 1.10,
        "light-on-dark {light} vs dark-on-light {dark}"
    );
}

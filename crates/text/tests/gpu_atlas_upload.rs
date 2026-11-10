//! GPU tier: a draw uploads only the part of the atlas page that changed
//! since the last upload, and the whole page when it is new or was
//! repacked; either way the pixels match a draw from a fresh atlas.

use vapors_gpu_harness::{Gpu, gpu, read_rgba8};
use vapors_renderer::clear;
use vapors_scene::{Color, Size};
use vapors_text::{
    AtlasConfig, AtlasError, BlendParams, Family, FontSystem, GlyphAtlas, Icon, IconDraw, IconPath,
    IconStroke, Layout, LayoutOptions, RenderTarget, Style, TextPipeline,
};

const WIDTH: u32 = 96;
const HEIGHT: u32 = 32;
/// Holds every 16 px Geist glyph used here with room to spare.
const LARGE_PAGE: u32 = 128;
/// Holds four 16 px capitals, not eight, so a second word evicts the first.
const SMALL_PAGE: u32 = 32;
/// The coverage bitmap of Geist "B" at 16 px, one byte per pixel, as the
/// atlas stored it when measured (`GlyphAtlas::get` rect width * height).
const B_GLYPH_BYTES: u64 = 120;
const WHITE: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};
const BLACK: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

/// A pipeline and atlas kept across draws, each draw into a fresh target.
struct Painter {
    gpu: Gpu,
    system: FontSystem,
    atlas: GlyphAtlas,
    pipeline: TextPipeline,
}

/// What one draw produced: the target's pixels and the atlas bytes uploaded.
struct Drawn {
    pixels: Vec<u8>,
    atlas_bytes: u64,
}

impl Painter {
    fn new(page_size: u32) -> Self {
        let gpu = gpu();
        let atlas = GlyphAtlas::new(AtlasConfig { page_size });
        let pipeline = TextPipeline::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm, &atlas);
        Self {
            gpu,
            system: FontSystem::bundle_only(),
            atlas,
            pipeline,
        }
    }

    /// Draw `text` in Geist 16 px in a new frame.
    fn text(&mut self, text: &str) -> Drawn {
        let style = Style::new(Family::Geist, 16.0);
        let layout = Layout::new(&mut self.system, text, &style, &LayoutOptions::default());
        self.frame(|pipeline, atlas, target| {
            pipeline.draw(target, atlas, &layout, [2.0, 4.0], BlendParams::default())
        })
    }

    /// Draw `first` then `second` in one frame, at different heights; the
    /// bytes reported are the second draw's, the first draw's are dropped.
    fn two_words(&mut self, first: &str, second: &str) -> Drawn {
        let style = Style::new(Family::Geist, 16.0);
        let options = LayoutOptions::default();
        let one = Layout::new(&mut self.system, first, &style, &options);
        let two = Layout::new(&mut self.system, second, &style, &options);
        self.frame(|pipeline, atlas, target| {
            pipeline.draw(target, atlas, &one, [2.0, 0.0], BlendParams::default())?;
            pipeline.take_work();
            pipeline.draw(target, atlas, &two, [2.0, 16.0], BlendParams::default())
        })
    }

    /// Draw `icon` at 16 px in black in a new frame.
    fn icon(&mut self, icon: &Icon) -> Drawn {
        let draw = IconDraw {
            icon,
            origin: [60, 8],
            size: [16, 16],
            colour: BLACK,
        };
        self.frame(|pipeline, atlas, target| pipeline.draw_icons(target, atlas, &[draw]))
    }

    /// Run `paint` into a fresh white target as a new atlas frame.
    fn frame(
        &mut self,
        paint: impl FnOnce(
            &mut TextPipeline,
            &mut GlyphAtlas,
            &RenderTarget<'_>,
        ) -> Result<(), AtlasError>,
    ) -> Drawn {
        let gpu = &self.gpu;
        self.atlas.begin_frame();
        let texture = clear(&gpu.device, &gpu.queue, WIDTH, HEIGHT, WHITE);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let target = target(gpu, &view);
        paint(&mut self.pipeline, &mut self.atlas, &target).expect("draws");
        Drawn {
            pixels: read_rgba8(&gpu.device, &gpu.queue, &texture),
            atlas_bytes: self.pipeline.take_work().texture_bytes,
        }
    }
}

fn target<'a>(gpu: &'a Gpu, view: &'a wgpu::TextureView) -> RenderTarget<'a> {
    RenderTarget {
        device: &gpu.device,
        queue: &gpu.queue,
        view,
        viewport: Size {
            width: WIDTH,
            height: HEIGHT,
        },
        scissor: None,
    }
}

fn diagonal_icon() -> Icon {
    let path = IconPath::parse("M2 2L14 14").expect("parses");
    let strokes = vec![IconStroke { path, width: 2.0 }];
    Icon::new([0.0, 0.0, 16.0, 16.0], strokes).expect("a valid icon")
}

#[test]
fn gpu_a_new_page_is_uploaded_whole() {
    // Given
    let mut painter = Painter::new(LARGE_PAGE);

    // When
    let drawn = painter.text("A");

    // Then
    assert_eq!(drawn.atlas_bytes, u64::from(LARGE_PAGE * LARGE_PAGE));
}

#[test]
fn gpu_a_draw_adding_one_glyph_uploads_exactly_that_glyphs_bytes() {
    // Given
    let mut painter = Painter::new(LARGE_PAGE);
    painter.text("A");

    // When
    let drawn = painter.text("AB");

    // Then
    assert_eq!(drawn.atlas_bytes, B_GLYPH_BYTES);
}

#[test]
fn gpu_a_draw_adding_no_glyph_uploads_nothing() {
    // Given
    let mut painter = Painter::new(LARGE_PAGE);
    painter.text("AB");

    // When
    let drawn = painter.text("AB");

    // Then
    assert_eq!(drawn.atlas_bytes, 0);
}

#[test]
fn gpu_glyphs_added_to_a_used_page_draw_as_from_a_fresh_atlas() {
    // Given
    let mut warm = Painter::new(LARGE_PAGE);
    warm.text("Ho");
    let fresh = Painter::new(LARGE_PAGE).text("Hello");

    // When
    let drawn = warm.text("Hello");

    // Then
    assert!(drawn.atlas_bytes < u64::from(LARGE_PAGE * LARGE_PAGE));
    assert!(
        drawn.pixels == fresh.pixels,
        "pixels differ from a fresh atlas"
    );
}

#[test]
fn gpu_an_icon_added_to_a_used_page_draws_as_from_a_fresh_atlas() {
    // Given
    let icon = diagonal_icon();
    let mut warm = Painter::new(LARGE_PAGE);
    warm.text("Ho");
    let fresh = Painter::new(LARGE_PAGE).icon(&icon);

    // When
    let drawn = warm.icon(&icon);

    // Then
    assert!(drawn.atlas_bytes < u64::from(LARGE_PAGE * LARGE_PAGE));
    assert!(
        drawn.pixels == fresh.pixels,
        "pixels differ from a fresh atlas"
    );
}

#[test]
fn gpu_a_repack_uploads_the_whole_page_and_draws_as_from_a_fresh_atlas() {
    // Given: the page full of last frame's glyphs
    let mut warm = Painter::new(SMALL_PAGE);
    warm.text("ABCD");
    let fresh = Painter::new(SMALL_PAGE).text("EFGH");

    // When
    let drawn = warm.text("EFGH");

    // Then
    assert!(warm.atlas.evictions() > 0, "the page was not repacked");
    assert_eq!(drawn.atlas_bytes, u64::from(SMALL_PAGE * SMALL_PAGE));
    assert!(
        drawn.pixels == fresh.pixels,
        "pixels differ from a fresh atlas"
    );
}

#[test]
fn gpu_a_repack_in_a_frame_that_already_added_a_glyph_uploads_the_whole_page() {
    // Given: three glyphs from last frame; the first draw adds a fourth,
    // so the second draw's three new glyphs only fit after a repack
    let mut warm = Painter::new(SMALL_PAGE);
    warm.text("ABC");
    let fresh = Painter::new(SMALL_PAGE).two_words("D", "EFG");

    // When
    let drawn = warm.two_words("D", "EFG");

    // Then
    assert!(warm.atlas.evictions() > 0, "the page was not repacked");
    assert_eq!(drawn.atlas_bytes, u64::from(SMALL_PAGE * SMALL_PAGE));
    assert!(
        drawn.pixels == fresh.pixels,
        "pixels differ from a fresh atlas"
    );
}

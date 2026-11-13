//! A text pipeline and atlas kept across draws, each into a fresh target.

// each test crate including this uses a part of it
#![allow(dead_code)]

use vapors_gpu_harness::{Gpu, gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, clear};
use vapors_scene::{Color, Size};
use vapors_text::{
    AtlasConfig, AtlasError, BlendParams, Family, FontSystem, GlyphAtlas, Icon, IconDraw, IconPath,
    IconStroke, Layout, LayoutOptions, RenderTarget, Style, TextPipeline,
};

pub const WIDTH: u32 = 96;
pub const HEIGHT: u32 = 32;
pub const WHITE: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};
pub const BLACK: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

/// A pipeline and atlas kept across draws, each draw into a fresh target.
pub struct Painter {
    pub gpu: Gpu,
    pub system: FontSystem,
    pub atlas: GlyphAtlas,
    pub pipeline: TextPipeline,
}

/// What one draw produced: the target's pixels and the atlas bytes uploaded.
pub struct Drawn {
    pub pixels: Vec<u8>,
    pub atlas_bytes: u64,
}

impl Painter {
    pub fn new(page_size: u32) -> Self {
        Self::with_atlas(AtlasConfig::fixed(page_size))
    }

    pub fn with_atlas(config: AtlasConfig) -> Self {
        let gpu = gpu();
        let atlas = GlyphAtlas::new(config);
        let pipeline = TextPipeline::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm, &atlas);
        Self {
            gpu,
            system: FontSystem::bundle_only(),
            atlas,
            pipeline,
        }
    }

    /// Draw `text` in Geist 16 px in a new frame.
    pub fn text(&mut self, text: &str) -> Drawn {
        let style = Style::new(Family::Geist, 16.0);
        let layout = Layout::new(&mut self.system, text, &style, &LayoutOptions::default());
        self.frame(|pipeline, atlas, target| {
            pipeline.draw(target, atlas, &layout, [2.0, 4.0], BlendParams::default())
        })
    }

    /// Draw `first` then `second` in one frame, at different heights; the
    /// bytes reported are the second draw's, the first draw's are dropped.
    pub fn two_words(&mut self, first: &str, second: &str) -> Drawn {
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
    pub fn icon(&mut self, icon: &Icon) -> Drawn {
        let draw = icon_draw(icon);
        self.frame(|pipeline, atlas, target| pipeline.draw_icons(target, atlas, &[draw]))
    }

    /// Draw `text` as [`Painter::text`] does, then `icon` as
    /// [`Painter::icon`] does, in one frame: two blends in one encoder.
    pub fn text_then_icon(&mut self, text: &str, icon: &Icon) -> Drawn {
        let style = Style::new(Family::Geist, 16.0);
        let layout = Layout::new(&mut self.system, text, &style, &LayoutOptions::default());
        let draw = icon_draw(icon);
        self.frame(|pipeline, atlas, target| {
            pipeline.draw(target, atlas, &layout, [2.0, 4.0], BlendParams::default())?;
            pipeline.draw_icons(target, atlas, &[draw])
        })
    }

    /// Run `paint` into a fresh white target as a new atlas frame.
    pub fn frame(
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
        let encoder = FrameEncoder::new(&gpu.device);
        let target = target(gpu, &encoder, &view);
        paint(&mut self.pipeline, &mut self.atlas, &target).expect("draws");
        encoder.submit(&gpu.queue).expect("no draw refused");
        Drawn {
            pixels: read_rgba8(&gpu.device, &gpu.queue, &texture),
            atlas_bytes: self.pipeline.take_work().texture_bytes,
        }
    }

    /// `text` laid out as [`Painter::text`] draws it.
    pub fn layout(&mut self, text: &str) -> Layout {
        let style = Style::new(Family::Geist, 16.0);
        Layout::new(&mut self.system, text, &style, &LayoutOptions::default())
    }
}

fn target<'a>(
    gpu: &'a Gpu,
    encoder: &'a FrameEncoder,
    view: &'a wgpu::TextureView,
) -> RenderTarget<'a> {
    RenderTarget {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder,
        view,
        viewport: Size {
            width: WIDTH,
            height: HEIGHT,
        },
        scissor: None,
    }
}

/// `icon` at 16 px in black, right of where the text is drawn.
fn icon_draw(icon: &Icon) -> IconDraw<'_> {
    IconDraw {
        icon,
        origin: [60, 8],
        size: [16, 16],
        colour: BLACK,
    }
}

pub fn diagonal_icon() -> Icon {
    let path = IconPath::parse("M2 2L14 14").expect("parses");
    let strokes = vec![IconStroke { path, width: 2.0 }];
    Icon::new([0.0, 0.0, 16.0, 16.0], strokes).expect("a valid icon")
}

//! Rendering the page and the composite's labels with this crate.

use vapors_renderer::{SolidPipeline, clear, read_rgba8};
use vapors_scene::Size;

use crate::{
    AtlasConfig, BlendParams, Family, FontSystem, GlyphAtlas, Layout, LayoutOptions, RenderTarget,
    Style, TextPipeline,
};

use super::measure::Rgba;
use super::spec::{Block, PADDING, PAGE_HEIGHT, PAGE_WIDTH, PANEL_WIDTH, SCHEMES, Scheme, blocks};

/// Render the whole page at `scale` device pixels per logical pixel:
/// 1 with light hinting, 2 with none (`Hinting::for_scale`).
///
/// # Panics
///
/// If a block cannot be drawn; the message is the atlas error, and a
/// fixed page of bundled fonts failing to draw is a bug, not an input.
#[must_use]
pub fn render_page(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    system: &mut FontSystem,
    params: BlendParams,
    scale: u32,
) -> Rgba {
    let viewport = Size {
        width: PAGE_WIDTH * scale,
        height: PAGE_HEIGHT * scale,
    };
    let (texture, view) = target(device, queue, viewport, &SCHEMES[0]);
    let mut text = Text::new(device, texture.format());
    let solid = SolidPipeline::new(device, texture.format());
    let render_target = RenderTarget {
        device,
        queue,
        view: &view,
        viewport,
    };
    for (index, scheme) in (0u32..).zip(SCHEMES.iter()) {
        let left = px(PANEL_WIDTH * index * scale);
        let rect = [
            left,
            0.0,
            left + px(PANEL_WIDTH * scale),
            px(viewport.height),
        ];
        solid.fill(device, queue, &view, viewport, rect, scheme.background);
        for block in blocks() {
            let layout = block_layout(system, &block, scheme, scale);
            let origin = [left + px(PADDING * scale), px(block.top * scale)];
            text.draw(&render_target, &layout, origin, params);
        }
    }
    Rgba {
        width: viewport.width,
        height: viewport.height,
        data: read_rgba8(device, queue, &texture),
    }
}

/// A `width` by 24 px strip naming the two columns of the composite.
#[must_use]
pub fn render_labels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    system: &mut FontSystem,
    width: u32,
    labels: [&str; 2],
) -> Rgba {
    let viewport = Size { width, height: 24 };
    let scheme = SCHEMES[0];
    let (texture, view) = target(device, queue, viewport, &scheme);
    let mut text = Text::new(device, texture.format());
    let render_target = RenderTarget {
        device,
        queue,
        view: &view,
        viewport,
    };
    let style = Style {
        color: scheme.ink,
        weight: 500.0,
        ..Style::new(Family::Geist, 13.0)
    };
    for (index, label) in (0u32..).zip(labels) {
        let layout = Layout::new(system, label, &style, &LayoutOptions::default());
        let x = px(width / 2 * index + 8);
        text.draw(&render_target, &layout, [x, 3.0], BlendParams::default());
    }
    Rgba {
        width,
        height: 24,
        data: read_rgba8(device, queue, &texture),
    }
}

/// Pixel counts to coordinates; the page is far below 2^24 pixels.
#[allow(clippy::cast_precision_loss)]
fn px(pixels: u32) -> f32 {
    pixels as f32
}

fn target(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    viewport: Size,
    scheme: &Scheme,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = clear(
        device,
        queue,
        viewport.width,
        viewport.height,
        scheme.background,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn block_layout(system: &mut FontSystem, block: &Block, scheme: &Scheme, scale: u32) -> Layout {
    let style = Style {
        weight: block.weight,
        tabular_figures: block.tabular_figures,
        line_height: block.line_height,
        color: scheme.ink,
        ..Style::new(block.family, block.size)
    };
    let options = LayoutOptions {
        width: Some(px(Block::width())),
        scale: px(scale),
        ..LayoutOptions::default()
    };
    Layout::new(system, block.text, &style, &options)
}

/// Pipeline and atlas together, for drawing several layouts into one target.
struct Text {
    pipeline: TextPipeline,
    atlas: GlyphAtlas,
}

impl Text {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let atlas = GlyphAtlas::new(AtlasConfig::default());
        Self {
            pipeline: TextPipeline::new(device, format, &atlas),
            atlas,
        }
    }

    fn draw(
        &mut self,
        target: &RenderTarget<'_>,
        layout: &Layout,
        origin: [f32; 2],
        params: BlendParams,
    ) {
        self.pipeline
            .draw(target, &mut self.atlas, layout, origin, params)
            .unwrap_or_else(|error| panic!("comparison page: {error}"));
    }
}

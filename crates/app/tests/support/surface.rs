//! Shared by the `gpu_surface*` tests: test renderers (`renderers.rs`,
//! `misfit.rs`), documents built from a short element list, and a
//! painter or an `ElementsApp` drawing into a readable target.

// each test crate including this uses a part of it
#![allow(dead_code, unused_imports)]

#[path = "app_frame.rs"]
mod app_frame;
#[path = "misfit.rs"]
mod misfit;
#[path = "renderers.rs"]
mod renderers;
pub use app_frame::{app_frame, app_frame_at};
pub use misfit::Misfit;
pub use renderers::{Log, Solid};

use vapors_app::{DrawOverError, PaintError, ScenePainter, Tooltip, TooltipError};
use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};
use vapors_gpu_harness::{Gpu, gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, Target, clear};
use vapors_scene::{Color, Size};
use vapors_text::FontSystem;

/// The target's side in device pixels.
pub const SIZE: u32 = 32;
pub const WHITE: [u8; 4] = [255, 255, 255, 255];
pub const RED: [u8; 4] = [255, 0, 0, 255];
pub const BLUE: [u8; 4] = [0, 0, 255, 255];

/// A document styled by `css` holding `elements`, each `(parent, tag,
/// class)` where `parent` indexes an earlier element (`None`: the body).
/// A `gpu-surface` is named after its class.
pub fn document(css: &str, elements: &[(Option<usize>, &str, &str)]) -> Document {
    let base = "html, body { margin: 0; background-color: #ffffff; } ";
    built(&format!("{base}{css}"), elements)
}

/// As [`document`], with no canvas background: for `draw_over`.
pub fn bare_document(css: &str, elements: &[(Option<usize>, &str, &str)]) -> Document {
    built(&format!("html, body {{ margin: 0; }} {css}"), elements)
}

fn built(sheet: &str, elements: &[(Option<usize>, &str, &str)]) -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("app.css", sheet).expect("parses");
    let mut made = Vec::new();
    for (parent, tag, class) in elements {
        let element = doc.create_element(tag);
        doc.set_attribute(element, "class", class);
        if *tag == "gpu-surface" {
            doc.set_attribute(element, "name", class);
        }
        let parent = parent.map_or_else(|| doc.body(), |at| made[at]);
        doc.append(parent, element);
        made.push(element);
    }
    doc
}

/// Updates `doc` for a `SIZE`-device-pixel square target at `scale`.
#[allow(clippy::cast_precision_loss)] // a small size
pub fn update(doc: &mut Document, scale: f32) {
    let side = SIZE as f32 / scale;
    let viewport = Viewport {
        width: side,
        height: side,
        scale,
    };
    doc.update(viewport).expect("updates");
}

/// A painter and the GPU it draws on.
pub struct Rig {
    pub gpu: Gpu,
    pub painter: ScenePainter,
}

impl Rig {
    pub fn new() -> Self {
        Self::on(gpu())
    }

    /// A painter on `gpu`.
    pub fn on(gpu: Gpu) -> Self {
        let painter =
            ScenePainter::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm).expect("Rgba8Unorm");
        Self { gpu, painter }
    }

    /// `doc` drawn into a fresh `SIZE` square target: its RGBA bytes.
    pub fn draw(&mut self, doc: &Document) -> Result<Vec<u8>, PaintError> {
        let painted = doc.painted().expect("updated");
        self.on_fresh_target(|painter, target| painter.draw(target, painted))
    }

    /// `doc` drawn over a fresh `SIZE` square target: its RGBA bytes.
    pub fn draw_over(&mut self, doc: &Document) -> Result<Vec<u8>, DrawOverError> {
        let painted = doc.painted().expect("updated");
        self.on_fresh_target(|painter, target| painter.draw_over(target, painted))
    }

    /// `doc` drawn into a fresh `SIZE` square target with `tooltip`
    /// over it at `pointer`: its RGBA bytes.
    pub fn draw_tooltip(
        &mut self,
        doc: &mut Document,
        tooltip: &Tooltip,
        pointer: Option<[f32; 2]>,
    ) -> Result<Vec<u8>, TooltipError> {
        self.on_fresh_target(|painter, target| {
            let painted = doc.painted().expect("updated");
            painter.draw(target, painted).expect("draws");
            painter.draw_tooltip(target, doc, tooltip, pointer)
        })
    }

    fn on_fresh_target<E>(
        &mut self,
        draw: impl FnOnce(&mut ScenePainter, &Target<'_>) -> Result<(), E>,
    ) -> Result<Vec<u8>, E> {
        let gpu = &self.gpu;
        let black = Color::from_rgba8(0, 0, 0, 255);
        let texture = clear(&gpu.device, &gpu.queue, SIZE, SIZE, black);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let encoder = FrameEncoder::new(&gpu.device);
        let drawn = draw(&mut self.painter, &target(gpu, &encoder, &view));
        encoder.submit(&gpu.queue).expect("no draw refused");
        drawn?;
        Ok(read_rgba8(&gpu.device, &gpu.queue, &texture))
    }
}

/// The whole of `view`, a `SIZE` square, as a target on `gpu`.
fn target<'a>(gpu: &'a Gpu, encoder: &'a FrameEncoder, view: &'a wgpu::TextureView) -> Target<'a> {
    let size = Size {
        width: SIZE,
        height: SIZE,
    };
    Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder,
        view,
        viewport: size,
        scissor: None,
    }
}

/// The pixel at device `(x, y)` of a `SIZE` square target's bytes.
pub fn pixel(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
    let at = ((y * SIZE + x) * 4) as usize;
    rgba[at..at + 4].try_into().expect("four bytes")
}

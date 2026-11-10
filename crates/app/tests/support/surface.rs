//! Shared by the `gpu_surface*` tests: a renderer that fills its surface
//! with one colour and logs the calls it gets, documents built from a
//! short element list, and a painter drawing into a readable target.

// each test crate including this uses a part of it
#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::{
    DrawOverError, PaintError, ScenePainter, SurfaceError, SurfaceFrame, SurfaceRenderer,
    SurfaceSize,
};
use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};
use vapors_gpu_harness::{Gpu, gpu, read_rgba8};
use vapors_renderer::{Target, clear};
use vapors_scene::{Color, Size};
use vapors_text::FontSystem;

/// The target's side in device pixels.
pub const SIZE: u32 = 32;
pub const WHITE: [u8; 4] = [255, 255, 255, 255];
pub const RED: [u8; 4] = [255, 0, 0, 255];
pub const BLUE: [u8; 4] = [0, 0, 255, 255];

/// What a [`Solid`] was asked to do.
#[derive(Default)]
pub struct Log {
    pub mounts: usize,
    pub resizes: Vec<SurfaceSize>,
    pub renders: usize,
    pub dropped: bool,
}

/// Fills its surface with `rgba`; fails its renders when `fails`.
pub struct Solid {
    pub rgba: [u8; 4],
    pub log: Rc<RefCell<Log>>,
    pub animating: bool,
    pub fails: bool,
}

impl Solid {
    /// An opaque `rgba` renderer and its log.
    pub fn new(rgba: [u8; 4]) -> (Self, Rc<RefCell<Log>>) {
        let log = Rc::new(RefCell::new(Log::default()));
        let solid = Self {
            rgba,
            log: Rc::clone(&log),
            animating: false,
            fails: false,
        };
        (solid, log)
    }
}

impl SurfaceRenderer for Solid {
    fn mount(&mut self, _device: &wgpu::Device) -> Result<(), SurfaceError> {
        self.log.borrow_mut().mounts += 1;
        Ok(())
    }

    fn resize(&mut self, _device: &wgpu::Device, size: SurfaceSize) -> Result<(), SurfaceError> {
        self.log.borrow_mut().resizes.push(size);
        Ok(())
    }

    fn render(&mut self, frame: SurfaceFrame<'_>) -> Result<(), SurfaceError> {
        if self.fails {
            return Err("the track's shader did not compile".into());
        }
        self.log.borrow_mut().renders += 1;
        let [r, g, b, a] = self.rgba.map(|c| f64::from(c) / 255.0);
        frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("solid"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        Ok(())
    }

    fn animating(&self) -> bool {
        self.animating
    }
}

impl Drop for Solid {
    fn drop(&mut self) {
        self.log.borrow_mut().dropped = true;
    }
}

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
        let gpu = gpu();
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

    fn on_fresh_target<E>(
        &mut self,
        draw: impl FnOnce(&mut ScenePainter, &Target<'_>) -> Result<(), E>,
    ) -> Result<Vec<u8>, E> {
        let gpu = &self.gpu;
        let black = Color::from_rgba8(0, 0, 0, 255);
        let texture = clear(&gpu.device, &gpu.queue, SIZE, SIZE, black);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let size = Size {
            width: SIZE,
            height: SIZE,
        };
        let target = Target {
            device: &gpu.device,
            queue: &gpu.queue,
            view: &view,
            viewport: size,
            scissor: None,
        };
        draw(&mut self.painter, &target)?;
        Ok(read_rgba8(&gpu.device, &gpu.queue, &texture))
    }
}

/// The pixel at device `(x, y)` of a `SIZE` square target's bytes.
pub fn pixel(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
    let at = ((y * SIZE + x) * 4) as usize;
    rgba[at..at + 4].try_into().expect("four bytes")
}

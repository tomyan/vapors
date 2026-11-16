//! Root module so the test ids start with `gpu_` (the tier split): what
//! `App::start` does that only the crate can see (one-scene F0): the
//! frame timer made before the first frame.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_gpu_harness::{Gpu, gpu_with};
use vapors_platform::{App, Start};
use vapors_renderer::TIMER_FEATURES;
use vapors_text::FontSystem;

use crate::{ElementsApp, SurfaceError, SurfaceFrame, SurfaceRenderer, SurfaceSize};

/// A renderer that draws nothing.
struct Blank;

impl SurfaceRenderer for Blank {
    fn mount(&mut self, _device: &wgpu::Device) -> Result<(), SurfaceError> {
        Ok(())
    }
    fn resize(&mut self, _device: &wgpu::Device, _size: SurfaceSize) -> Result<(), SurfaceError> {
        Ok(())
    }
    fn render(&mut self, _frame: SurfaceFrame<'_>) -> Result<(), SurfaceError> {
        Ok(())
    }
}

/// A timed app showing one 16 x 16 surface named `track`, and its GPU.
fn rig() -> (ElementsApp, Gpu) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("app.css", "gpu-surface { width: 16px; height: 16px; }")
        .expect("parses");
    let surface = doc.create_element("gpu-surface");
    doc.set_attribute(surface, "name", "track");
    let body = doc.body();
    doc.append(body, surface);
    let mut app = ElementsApp::new(Rc::new(RefCell::new(doc)));
    app.time_frames(true);
    app.mount("track", Box::new(Blank)).expect("mounts");
    (app, gpu_with(TIMER_FEATURES))
}

fn start(app: &mut ElementsApp, gpu: &Gpu) {
    app.start(&Start {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        format: wgpu::TextureFormat::Rgba8Unorm,
        width: 32,
        height: 32,
        scale_factor: 1.0,
    })
    .expect("starts");
}

#[test]
fn gpu_start_makes_the_frame_timer_of_a_timed_app() {
    // Given
    let (mut app, gpu) = rig();

    // When
    start(&mut app, &gpu);

    // Then
    assert!(app.timer_made());
}

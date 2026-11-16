//! The app the `gpu_app_start*` tests start: a 16 x 16 surface named
//! `track` over a line of text, and starts at the test target's size.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_dom::Dom;
use vapors_gpu_harness::{Gpu, gpu};
use vapors_platform::{App, Start};

use super::{Log, RED, SIZE, Solid, document};

const CSS: &str = ".track { width: 16px; height: 16px; } p { font-size: 8px; margin: 0; }";

/// An app showing a 16 x 16 surface named `track` over a line of text,
/// its renderer's log, and its GPU.
pub fn rig() -> (ElementsApp, Rc<RefCell<Log>>, Gpu) {
    let mut doc = document(CSS, &[(None, "gpu-surface", "track"), (None, "p", "")]);
    let line = doc.create_text("Lap 1");
    let paragraph = doc.children(doc.body())[1];
    doc.append(paragraph, line);
    let mut app = ElementsApp::new(Rc::new(RefCell::new(doc)));
    let (solid, log) = Solid::new(RED);
    app.mount("track", Box::new(solid)).expect("mounts");
    (app, log, gpu())
}

/// `app`'s start on `gpu`, at the test target's size and format.
pub fn start(app: &mut ElementsApp, gpu: &Gpu) -> Result<(), String> {
    start_for(app, gpu, wgpu::TextureFormat::Rgba8Unorm)
}

/// `app`'s start on `gpu` for textures of `format`.
pub fn start_for(
    app: &mut ElementsApp,
    gpu: &Gpu,
    format: wgpu::TextureFormat,
) -> Result<(), String> {
    app.start(&Start {
        format,
        ..start_at(gpu)
    })
}

/// A start on `gpu` for the test target: its size and format.
pub fn start_at(gpu: &Gpu) -> Start<'_> {
    Start {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        format: wgpu::TextureFormat::Rgba8Unorm,
        width: SIZE,
        height: SIZE,
        scale_factor: 1.0,
    }
}

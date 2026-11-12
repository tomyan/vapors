//! Two panels holding `track` (red) and `map` (blue), shared by the
//! `gpu_app_two_panels*` tests.

// each test crate including this uses a part of it
#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::{BLUE, Log, RED, Solid, app_frame, document};
use vapors_app::{ElementsApp, SurfaceHandle};
use vapors_elements::Document;
use vapors_gpu_harness::{Gpu, gpu};
use vapors_platform::Next;

/// Panels at x 2..14 and 18..30, y 2..14, each clipping a 24 x 24
/// surface; `.bar` crosses both at y 6..8, painted after them.
pub const OVERSIZED: &str = "body { display: flex; } \
    .left, .right { width: 12px; height: 12px; margin: 2px; overflow: hidden; } \
    .track, .map { width: 24px; height: 24px; } \
    .bar { position: absolute; left: 0; top: 6px; width: 32px; height: 2px; \
    background-color: #00ff00; }";

/// The right panel takes what the left leaves; each surface fills its
/// panel. `.narrow` takes the left panel from 12 to 8 pixels wide.
pub const FLEXIBLE: &str = "body { display: flex; } \
    .left, .right { width: 12px; height: 12px; margin: 2px; } \
    .left.narrow { width: 8px; } .right { flex-grow: 1; } \
    .track, .map { width: 100%; height: 100%; }";

/// Two panels holding `track` (red) and `map` (blue), and a bar when
/// `bar`.
pub struct Panels {
    pub app: ElementsApp,
    pub doc: Rc<RefCell<Document>>,
    pub gpu: Gpu,
    pub track: (SurfaceHandle, Rc<RefCell<Log>>),
    pub map: (SurfaceHandle, Rc<RefCell<Log>>),
}

impl Panels {
    pub fn new(css: &str, bar: bool) -> Self {
        let mut elements = vec![
            (None, "div", "left"),
            (Some(0), "gpu-surface", "track"),
            (None, "div", "right"),
            (Some(2), "gpu-surface", "map"),
        ];
        if bar {
            elements.push((None, "div", "bar"));
        }
        let doc = Rc::new(RefCell::new(document(css, &elements)));
        let mut app = ElementsApp::new(Rc::clone(&doc));
        let track = mount(&mut app, "track", RED);
        let map = mount(&mut app, "map", BLUE);
        let gpu = gpu();
        Self {
            app,
            doc,
            gpu,
            track,
            map,
        }
    }

    /// `count` frames: what each asked for next; the last one's bytes.
    pub fn frames(&mut self, count: usize) -> (Vec<Next>, Vec<u8>) {
        let mut asked = Vec::new();
        let mut rgba = Vec::new();
        for _ in 0..count {
            let (next, bytes) = app_frame(&mut self.app, &self.gpu);
            asked.push(next);
            rgba = bytes;
        }
        (asked, rgba)
    }

    /// Renders so far: `track`'s and `map`'s.
    pub fn renders(&self) -> (usize, usize) {
        let count = |log: &Rc<RefCell<Log>>| log.borrow().frames.len();
        (count(&self.track.1), count(&self.map.1))
    }

    /// Both surfaces animating, after a first frame.
    pub fn animating(css: &str) -> Self {
        let mut panels = Self::new(css, false);
        panels.frames(1);
        panels.track.0.request_animation().expect("mounted");
        panels.map.0.request_animation().expect("mounted");
        panels
    }
}

fn mount(app: &mut ElementsApp, name: &str, rgba: [u8; 4]) -> (SurfaceHandle, Rc<RefCell<Log>>) {
    let (solid, log) = Solid::new(rgba);
    (app.mount(name, Box::new(solid)).expect("mounts"), log)
}

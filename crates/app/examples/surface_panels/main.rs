//! Two `gpu-surface`s in two panels, each with its own gradient renderer
//! and its own `SurfaceHandle`:
//!
//! ```text
//! cargo run -p vapors-app --example surface_panels
//! ```
//!
//! The left surface animates throughout. The right one animates for
//! 2 s, is stopped for 2 s, and so on, while the left keeps moving.
//! The timing is checked by the unit tests in `flip.rs`, the rest by the
//! GPU tests in `tests/gpu_app_two_panels.rs`.

mod flip;
#[path = "../surface_gradient/gradient.rs"]
mod gradient;
mod page;

use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use vapors_app::{ElementsApp, InvalidateError, SurfaceHandle};
use vapors_platform::{App, Event, Frame, Instant, Next, Platform, WindowOptions, run};

use flip::{Flip, Flipper};
use gradient::Gradient;

fn main() -> ExitCode {
    match start() {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!(
                "surface_panels: {why}\nreproduce: cargo run -p vapors-app --example surface_panels"
            );
            ExitCode::FAILURE
        }
    }
}

fn start() -> Result<(), String> {
    let mut app = ElementsApp::new(page::document());
    let [left, right] = page::SURFACES.map(|name| {
        let gradient = Gradient::counting(Arc::new(AtomicU64::new(0)));
        app.mount(name, Box::new(gradient))
    });
    let (left, right) = (
        left.map_err(|e| e.to_string())?,
        right.map_err(|e| e.to_string())?,
    );
    left.request_animation().map_err(|e| e.to_string())?;
    let failure = app.failure();
    let options = WindowOptions {
        app_name: "vapors".into(),
        title: "vapors: two surface panels".into(),
        width: 800,
        height: 400,
    };
    run(options, Alternating::new(app, right)).map_err(|error| error.to_string())?;
    failure.get().map_or(Ok(()), Err)
}

/// The app, starting and stopping the right surface's animation every
/// `PHASE`.
struct Alternating {
    inner: ElementsApp,
    right: SurfaceHandle,
    flipper: Flipper,
}

impl Alternating {
    fn new(inner: ElementsApp, right: SurfaceHandle) -> Self {
        Self {
            inner,
            right,
            flipper: Flipper::default(),
        }
    }

    fn turn(&mut self, now: Instant) -> Result<(), InvalidateError> {
        match self.flipper.at(now) {
            Flip::Start => self.right.request_animation().map(drop),
            Flip::Stop => self.right.stop_animation(),
            Flip::Keep => Ok(()),
        }
    }
}

impl App for Alternating {
    fn event(&mut self, event: Event, platform: &Platform) {
        self.inner.event(event, platform);
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        if let Err(error) = self.turn(frame.now) {
            eprintln!("surface_panels: the right surface's handle refused: {error}");
            std::process::exit(1);
        }
        self.inner.frame(frame)
    }

    fn drawn(&mut self, platform: &Platform) {
        self.inner.drawn(platform);
    }
}

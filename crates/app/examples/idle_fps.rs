//! Frames an app draws while nothing happens:
//!
//! ```text
//! cargo run -p vapors-app --example idle_fps -- hello [seconds]
//! cargo run -p vapors-app --example idle_fps -- image [seconds]
//! ```
//!
//! Opens the window, waits for the first frame, then counts the frames
//! presented over `seconds` (default 3) with no input, prints the count
//! and the rate, and exits. An app that redraws only on change draws
//! none. Native only: it ends the process from a timer thread, because
//! an idle app gets no callback to end it from.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use vapors_app::ElementsApp;
use vapors_canvas::{ImageApp, decode_png};
use vapors_platform::{App, Event, Frame, Next, Platform, WindowOptions, run};
use vapors_scene::Color;

const GRADIENT: &[u8] = include_bytes!("../../platform/examples/fixtures/gradient.png");

/// Counts frames after the first, and starts the timer at the first.
struct Counted<A> {
    app: A,
    frames: Arc<AtomicU64>,
    seconds: u64,
    started: bool,
}

impl<A: App> App for Counted<A> {
    fn event(&mut self, event: Event, platform: &Platform) {
        self.app.event(event, platform);
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        if self.started {
            self.frames.fetch_add(1, Ordering::SeqCst);
        } else {
            self.started = true;
            start_timer(Arc::clone(&self.frames), self.seconds);
        }
        self.app.frame(frame)
    }
}

#[allow(clippy::cast_precision_loss)] // frame counts are small
fn start_timer(frames: Arc<AtomicU64>, seconds: u64) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(seconds));
        let count = frames.load(Ordering::SeqCst);
        println!(
            "idle frames in {seconds} s: {count} ({:.1} per second)",
            count as f64 / seconds as f64
        );
        std::process::exit(0);
    });
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let name = args.first().cloned().unwrap_or_else(|| "hello".to_owned());
    let seconds = args
        .get(1)
        .map_or(Ok(3), |s| s.parse())
        .map_err(|_| "seconds is not a whole number")?;
    let options = WindowOptions {
        app_name: "idle_fps".into(),
        title: format!("idle fps: {name}"),
        width: 640,
        height: 400,
    };
    let frames = Arc::new(AtomicU64::new(0));
    if name == "image" {
        let image = decode_png(GRADIENT)?;
        let app = ImageApp::showing("gradient.png", image, Color::from_rgba8(40, 40, 40, 255));
        return run(
            options,
            Counted {
                app,
                frames,
                seconds,
                started: false,
            },
        )
        .map_err(|e| e.to_string());
    }
    let demo = vapors_demos::find(&name)?;
    let app = ElementsApp::new(vapors_demos::document(&demo)?);
    run(
        options,
        Counted {
            app,
            frames,
            seconds,
            started: false,
        },
    )
    .map_err(|e| e.to_string())
}

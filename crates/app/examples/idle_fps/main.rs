//! Frames an app draws while nothing happens:
//!
//! ```text
//! cargo run -p vapors-app --example idle_fps -- hello [seconds]
//! cargo run -p vapors-app --example idle_fps -- image [seconds]
//! cargo run -p vapors-app --example idle_fps -- task-pending|task-quiet|task-change [seconds]
//! ```
//!
//! Opens the window, waits for the first frame, then counts the frames
//! presented over `seconds` (default 3) with no input, prints the count
//! and the rate, and exits. An app that redraws only on change draws
//! none. Native only: it ends the process from a timer thread, because
//! an idle app gets no callback to end it from.
//!
//! The task cases (`docs/plans/async-components.md` T2) show a count
//! and spawn one task: `task-pending` is polled once and never woken
//! (0 frames); `task-quiet` is woken by a thread every 500 ms and writes
//! nothing (0 frames); `task-change` is woken the same and writes the
//! shown count (one frame per wake). The wakes are printed beside the
//! frames; `task-quiet` and `task-change` exit 1 with fewer than
//! `MIN_WAKES`, which shows a task wake never reached the task.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use vapors_app::ElementsApp;
use vapors_canvas::{ImageApp, decode_png};
use vapors_platform::{App, Event, Frame, Next, Platform, WindowOptions, run};
use vapors_scene::Color;

use tasks::{MIN_WAKES, WAKING, task_app};

mod tasks;

const GRADIENT: &[u8] = include_bytes!("../../../platform/examples/fixtures/gradient.png");

/// Counts frames after the first, and starts the timer at the first.
struct Counted<A> {
    app: A,
    frames: Arc<AtomicU64>,
    wakes: Arc<AtomicU64>,
    /// A waking task case: fewer than `MIN_WAKES` fails the run.
    wakes_expected: bool,
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
            let counts = (Arc::clone(&self.frames), Arc::clone(&self.wakes));
            start_timer(counts, self.wakes_expected, self.seconds);
        }
        self.app.frame(frame)
    }

    // the loop's task waker reaches the app here and in `tasks_woken`
    fn drawn(&mut self, platform: &Platform) {
        self.app.drawn(platform);
    }

    fn tasks_woken(&mut self, platform: &Platform) {
        self.app.tasks_woken(platform);
    }
}

#[allow(clippy::cast_precision_loss)] // frame counts are small
fn start_timer(
    (frames, wakes): (Arc<AtomicU64>, Arc<AtomicU64>),
    wakes_expected: bool,
    seconds: u64,
) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(seconds));
        let (count, woke) = (frames.load(Ordering::SeqCst), wakes.load(Ordering::SeqCst));
        println!(
            "idle frames in {seconds} s: {count} ({:.1} per second), task wakes: {woke}",
            count as f64 / seconds as f64,
        );
        if wakes_expected && woke < MIN_WAKES {
            eprintln!("{FEW_WAKES}");
            std::process::exit(1);
        }
        std::process::exit(0);
    });
}

const FEW_WAKES: &str = "idle_fps: fewer than 2 task wakes reached the task, so a task wake \
    did not poll it; fix: check that Platform::task_waker posts Wake::Tasks and that every \
    App wrapping the ElementsApp forwards App::tasks_woken";

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
        decorations: true,
        app_id: None,
    };
    let wakes = Arc::new(AtomicU64::new(0));
    if name == "image" {
        let image = decode_png(GRADIENT)?;
        let app = ImageApp::showing("gradient.png", image, Color::from_rgba8(40, 40, 40, 255));
        return run(options, counted(app, (wakes, false), seconds)).map_err(|e| e.to_string());
    }
    let app = if let Some(app) = task_app(&name, &wakes) {
        app
    } else {
        let shown = vapors_demos::document(&vapors_demos::find(&name)?);
        ElementsApp::new(shown.document).with_per_frame(shown.per_frame)
    };
    let wakes_expected = WAKING.contains(&name.as_str());
    run(options, counted(app, (wakes, wakes_expected), seconds)).map_err(|e| e.to_string())
}

fn counted<A>(app: A, (wakes, wakes_expected): (Arc<AtomicU64>, bool), seconds: u64) -> Counted<A> {
    Counted {
        app,
        frames: Arc::default(),
        wakes,
        wakes_expected,
        seconds,
        started: false,
    }
}

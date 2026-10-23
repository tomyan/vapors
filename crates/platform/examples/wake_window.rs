//! The waker natively: an app that returns `Next::Wait` draws nothing
//! while idle and exactly one frame per `Waker::wake` from another
//! thread. `xvfb-run -a cargo run -p vapors-platform --example
//! wake_window` (the CI window job runs it). Exit 1 names what differed;
//! the process ends from the script's thread, since an idle app gets no
//! callback to end it from.

#[path = "support/mod.rs"]
mod support;

use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use vapors_platform::{App, Event, Frame, Next, Platform, Waker, WindowOptions, run};
use vapors_scene::Color;

/// After the first frame, for the window's own start-up frames (shown,
/// sized) to end; then the idle count runs.
const SETTLE: Duration = Duration::from_millis(500);
/// How long the app is left alone, expecting no frame.
const IDLE: Duration = Duration::from_secs(1);
/// Between wakes: llvmpipe draws this window in well under 50 ms.
const BETWEEN: Duration = Duration::from_millis(300);
const WAKES: usize = 3;

struct Woken {
    frames: Arc<AtomicU64>,
    waker: Option<Waker>,
    started: bool,
}

impl App for Woken {
    fn event(&mut self, _event: Event, platform: &Platform) {
        self.waker.get_or_insert_with(|| platform.waker());
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        support::clear(frame, Color::from_rgba8(12, 34, 200, 255));
        self.frames.fetch_add(1, Ordering::SeqCst);
        if !self.started {
            self.started = true;
            let Some(waker) = self.waker.clone() else {
                finish(Err(
                    "no event came before the first frame, so there was no waker".into(),
                ));
            };
            let frames = Arc::clone(&self.frames);
            std::thread::spawn(move || finish(script(&waker, &frames)));
        }
        Next::Wait
    }
}

/// Idle, then wake `WAKES` times; the frames each drew.
fn script(waker: &Waker, frames: &AtomicU64) -> Result<String, String> {
    std::thread::sleep(SETTLE);
    let before = frames.load(Ordering::SeqCst);
    std::thread::sleep(IDLE);
    let idle = frames.load(Ordering::SeqCst) - before;
    let per_wake: Vec<u64> = (0..WAKES)
        .map(|_| {
            let at = frames.load(Ordering::SeqCst);
            waker.wake();
            std::thread::sleep(BETWEEN);
            frames.load(Ordering::SeqCst) - at
        })
        .collect();
    let line = format!("idle frames in {IDLE:?}: {idle}; frames per wake: {per_wake:?}");
    if idle == 0 && per_wake.iter().all(|n| *n == 1) {
        Ok(line)
    } else {
        Err(format!("{line}; expected 0 idle and 1 per wake"))
    }
}

fn finish(outcome: Result<String, String>) -> ! {
    match outcome {
        Ok(line) => {
            println!("ok   wake_window: {line}");
            std::process::exit(0);
        }
        Err(why) => {
            eprintln!(
                "wake_window: {why}\nreproduce: xvfb-run -a cargo run -p vapors-platform --example wake_window"
            );
            std::process::exit(1);
        }
    }
}

fn main() -> ExitCode {
    let app = Woken {
        frames: Arc::new(AtomicU64::new(0)),
        waker: None,
        started: false,
    };
    let options = WindowOptions {
        app_name: "vapors".into(),
        title: "vapors: wake window".into(),
        width: 320,
        height: 200,
    };
    support::exit_code(run(options, app), "wake_window")
}

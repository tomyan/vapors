//! The waker natively: an app that returns `Next::Wait` draws nothing
//! while idle, exactly one frame per `Waker::wake` from another thread,
//! at most two for a burst of five, and once the loop has ended a wake
//! says `LoopClosed`. `xvfb-run -a cargo run -p vapors-platform
//! --example wake_window` (the CI window job runs it). Exit 1 names
//! what differed.

mod script;
#[path = "../support/mod.rs"]
mod support;

use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use vapors_platform::{App, Event, Frame, Next, Platform, Waker, WindowOptions, run};
use vapors_scene::Color;

use script::Shared;

struct Woken {
    shared: Arc<Shared>,
    waker: Arc<Mutex<Option<Waker>>>,
    started: bool,
}

impl App for Woken {
    fn event(&mut self, _event: Event, platform: &Platform) {
        let mut kept = self.waker.lock().expect("not poisoned");
        kept.get_or_insert_with(|| platform.waker());
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        support::clear(frame, Color::from_rgba8(12, 34, 200, 255));
        self.shared.drew();
        if !self.started {
            self.started = true;
            self.start_script();
        }
        if self.shared.exit_asked() {
            Next::Exit
        } else {
            Next::Wait
        }
    }
}

impl Woken {
    fn start_script(&self) {
        let waker = self.waker.lock().expect("not poisoned").clone();
        let shared = Arc::clone(&self.shared);
        std::thread::spawn(move || match waker {
            Some(waker) => script::run(&waker, &shared),
            None => shared.finish(Err(
                "no event came before the first frame, so there was no waker".into(),
            )),
        });
    }
}

fn main() -> ExitCode {
    let shared = Arc::new(Shared::default());
    let waker = Arc::new(Mutex::new(None));
    let app = Woken {
        shared: Arc::clone(&shared),
        waker: Arc::clone(&waker),
        started: false,
    };
    let options = WindowOptions {
        app_name: "vapors".into(),
        title: "vapors: wake window".into(),
        width: 320,
        height: 200,
    };
    if let Err(error) = run(options, app) {
        eprintln!("wake_window: {error}");
        return ExitCode::FAILURE;
    }
    let after_end = waker
        .lock()
        .expect("not poisoned")
        .as_ref()
        .map(Waker::wake);
    report(shared.outcome(), after_end)
}

/// The script's outcome, and a wake after the loop ended, which must be
/// `LoopClosed`.
fn report(
    outcome: Result<String, String>,
    after_end: Option<Result<(), vapors_platform::LoopClosed>>,
) -> ExitCode {
    let closed = matches!(after_end, Some(Err(vapors_platform::LoopClosed)));
    match (outcome, closed) {
        (Ok(line), true) => {
            println!("ok   wake_window: {line}; a wake after the loop ended: LoopClosed");
            ExitCode::SUCCESS
        }
        (outcome, _) => {
            let why = outcome.err().unwrap_or_else(|| {
                format!("a wake after the loop ended gave {after_end:?}, not LoopClosed")
            });
            eprintln!(
                "wake_window: {why}\nreproduce: xvfb-run -a cargo run -p vapors-platform --example wake_window"
            );
            ExitCode::FAILURE
        }
    }
}

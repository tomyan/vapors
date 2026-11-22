//! The Live timer on the desktop application thread
//! (`docs/plans/desktop-app-thread.md`, L1b).
//!
//! ```text
//! cargo run -p vapors-app --example live_timer              # Space toggles Live
//! xvfb-run -a cargo run -p vapors-app --example live_timer -- --check
//! ```
//!
//! Winit owns this thread and the window; the Live switch's handler and
//! the 100 ms Tokio interval run on the application thread
//! (`vapors_app::AppThread`), which shows each tick by a frame wake. With
//! `--check`, a script on the application thread runs Live off, on (with
//! a 350 ms stall of the thread), and off again, and the run exits 1
//! listing every rule it broke: handlers and tick continuations on the
//! application thread, the UI on another; no frames or ticks with Live
//! off; a frame per tick with Live on; no burst of ticks after the stall.

mod app_side;
mod check;
mod record;
mod script;
mod ui;

use std::cell::Cell;
use std::process::ExitCode;
use std::rc::Rc;
use std::time::Duration;

use vapors_platform::{WindowOptions, run};

/// The Live interval (the gui-bench dashboard's `LIVE_INTERVAL_MS`).
pub const TICK: Duration = Duration::from_millis(100);

/// How long `--check` may take: its script is 3.35 s and the window's
/// start under Xvfb about 1 s (measured below); 15 s is a hang.
const CHECK_DEADLINE: Duration = Duration::from_secs(15);

fn main() -> ExitCode {
    let check = std::env::args().any(|arg| arg == "--check");
    if check {
        watchdog();
    }
    let outcome = Rc::new(Cell::new(None));
    let options = WindowOptions {
        app_name: "live_timer".into(),
        title: "live timer: Space toggles Live".into(),
        width: 320,
        height: 200,
        decorations: true,
        app_id: None,
    };
    if let Err(error) = run(options, ui::Window::new(check, Rc::clone(&outcome))) {
        eprintln!("live_timer: {error}");
        return ExitCode::FAILURE;
    }
    match (check, outcome.get()) {
        (false, _) | (true, Some(true)) => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}

/// Ends the process if `--check` has not judged within its deadline.
fn watchdog() {
    std::thread::spawn(|| {
        std::thread::sleep(CHECK_DEADLINE);
        eprintln!(
            "live_timer --check: did not finish within {CHECK_DEADLINE:?}; fix: the phase that hung, from the last ticks printed"
        );
        std::process::exit(1);
    });
}

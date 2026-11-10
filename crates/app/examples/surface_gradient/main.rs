//! A WGSL gradient in a `gpu-surface` beside ordinary UI:
//!
//! ```text
//! cargo run -p vapors-app --example surface_gradient
//! xvfb-run -a cargo run -p vapors-app --example surface_gradient -- --check
//! ```
//!
//! With `--check` (the CI window job runs it) it settles, then counts
//! the frames and renders while idle, which must be none; invalidates
//! the surface from the running app, which must draw 2 frames and
//! render once; ends the loop; and checks that an invalidate after the loop ended is refused as
//! `LoopClosed`. Exit 1 names what differed.

mod check;
mod gradient;
mod page;

use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use vapors_app::{ElementsApp, Failure, InvalidateError, SurfaceHandle};
use vapors_platform::{App, WindowOptions, run};

use check::{Checked, Counts};
use gradient::Gradient;

const REPRODUCE: &str = "xvfb-run -a cargo run -p vapors-app --example surface_gradient -- --check";

fn main() -> ExitCode {
    let checking = std::env::args().nth(1).as_deref() == Some("--check");
    match start(checking) {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("surface_gradient: {why}\nreproduce: {REPRODUCE}");
            ExitCode::FAILURE
        }
    }
}

fn start(checking: bool) -> Result<String, String> {
    let renders = Arc::new(AtomicU64::new(0));
    let mut app = ElementsApp::new(page::document());
    let gradient = Gradient::counting(Arc::clone(&renders));
    let handle = app
        .mount(page::SURFACE, Box::new(gradient))
        .map_err(|error| error.to_string())?;
    let failure = app.failure();
    if checking {
        let counts = Counts::new(renders);
        let checked = Checked::new(app, handle.clone(), Arc::clone(&counts));
        open(checked, &failure)?;
        let idle = counts.outcome()?;
        after_the_loop(&handle)?;
        return Ok(format!(
            "ok   surface_gradient: {idle}; an invalidate after the loop ended: LoopClosed"
        ));
    }
    open(app, &failure).map(|()| String::new())
}

/// Runs `app` in a window until it ends; refused when it failed.
fn open(app: impl App + 'static, failure: &Failure) -> Result<(), String> {
    let options = WindowOptions {
        app_name: "vapors".into(),
        title: "vapors: surface gradient".into(),
        width: 640,
        height: 400,
    };
    run(options, app).map_err(|error| error.to_string())?;
    failure.get().map_or(Ok(()), Err)
}

/// The handle's invalidate once `run` has returned: `LoopClosed`.
fn after_the_loop(handle: &SurfaceHandle) -> Result<(), String> {
    match handle.invalidate() {
        Err(InvalidateError::LoopClosed(_)) => Ok(()),
        other => Err(format!(
            "an invalidate after the loop ended gave {other:?}, not LoopClosed"
        )),
    }
}

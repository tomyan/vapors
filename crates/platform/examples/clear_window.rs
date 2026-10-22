//! Open a window, clear it to one colour, and with `--screenshot out.png`
//! save the second frame (the first has been presented by then) and exit.
//! `cargo xtask screenshot clear_window out.png` runs this under Xvfb.

#[path = "support/mod.rs"]
mod support;

use std::path::PathBuf;
use std::process::ExitCode;

use vapors_platform::{App, Event, Frame, Next, Platform, WindowOptions, run};
use vapors_scene::Color;

const CLEAR: Color = Color {
    r: 12.0 / 255.0,
    g: 200.0 / 255.0,
    b: 34.0 / 255.0,
    a: 1.0,
};

struct ClearWindow {
    screenshot: Option<PathBuf>,
}

impl App for ClearWindow {
    fn event(&mut self, _event: Event, _platform: &Platform) {}

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        support::clear(frame, CLEAR);
        support::after_first_frame(frame, self.screenshot.as_deref())
    }
}

fn main() -> ExitCode {
    let app = ClearWindow {
        screenshot: support::flag_value("--screenshot").map(PathBuf::from),
    };
    let options = WindowOptions {
        app_name: "vapors".into(),
        title: "vapors: clear window".into(),
        width: 640,
        height: 400,
    };
    support::exit_code(run(options, app), "clear_window")
}

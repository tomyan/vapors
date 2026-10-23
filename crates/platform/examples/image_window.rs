//! Show a PNG fitted to the window. `--open <file.png>` shows it from the
//! start; otherwise, or any time, `o` or a click on the empty window opens
//! the file picker. Scroll, pinch or drag to steer; `0` fits, `1` is
//! actual size, `+` and `-` zoom about the centre, a double tap toggles
//! fit and actual size. `--screenshot out.png` saves the second frame
//! and exits.

#[path = "support/mod.rs"]
mod support;

use std::path::PathBuf;
use std::process::ExitCode;

use vapors_canvas::{ImageApp, decode_png};
use vapors_platform::{App, Event, Frame, Next, Platform, WindowOptions, run};
use vapors_scene::Color;

const BACKGROUND: Color = Color {
    r: 40.0 / 255.0,
    g: 40.0 / 255.0,
    b: 40.0 / 255.0,
    a: 1.0,
};

/// The shared image app plus this example's screenshot exit.
struct ImageWindow {
    app: ImageApp,
    screenshot: Option<PathBuf>,
}

impl App for ImageWindow {
    fn event(&mut self, event: Event, platform: &Platform) {
        self.app.event(event, platform);
    }

    /// The app's own answer, except that a screenshot run keeps
    /// drawing until it has its second frame.
    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        let next = self.app.frame(frame);
        if self.screenshot.is_none() {
            return next;
        }
        support::after_first_frame(frame, self.screenshot.as_deref())
    }
}

fn main() -> ExitCode {
    let path = support::flag_value("--open");
    let app = match path.as_deref().map(open) {
        None => ImageApp::new(BACKGROUND),
        Some(Ok(image)) => ImageApp::showing(&file_name(path.as_deref()), image, BACKGROUND),
        Some(Err(message)) => {
            eprintln!("image_window: {message}");
            return ExitCode::FAILURE;
        }
    };
    let window = ImageWindow {
        app,
        screenshot: support::flag_value("--screenshot").map(PathBuf::from),
    };
    let options = WindowOptions {
        app_name: "vapors".into(),
        title: "vapors: image window".into(),
        width: 640,
        height: 400,
    };
    support::exit_code(run(options, window), "image_window")
}

/// The last component of `path`, for the label.
fn file_name(path: Option<&str>) -> String {
    path.and_then(|p| std::path::Path::new(p).file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Read and decode the PNG at `path`; messages name the file.
fn open(path: &str) -> Result<vapors_canvas::RgbaImage, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("could not open {path}: {e}"))?;
    decode_png(&bytes).map_err(|message| format!("{path}: {message}"))
}

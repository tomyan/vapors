//! A demo in a window:
//!
//! ```text
//! cargo run -p vapors-app --example demo -- hello
//! cargo xtask screenshot demo target/hello.png -- hello
//! ```
//!
//! With `--screenshot <png>` (which `cargo xtask screenshot` adds) it
//! saves the second frame, checks it against the demo's `.pixels`, and
//! exits 1 when a check fails or the document was refused.

use std::cell::RefCell;
use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_demos::pixels::{Image, failures, parse};
use vapors_platform::{App, Event, Frame, Next, Platform, WindowOptions, run};

fn main() -> ExitCode {
    match start() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("demo: {message}");
            ExitCode::FAILURE
        }
    }
}

fn start() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let name = args
        .first()
        .filter(|a| !a.starts_with("--"))
        .ok_or("usage: demo <name> [--screenshot <png>]")?;
    let demo = vapors_demos::find(name)?;
    let app = ElementsApp::new(vapors_demos::document(&demo)?);
    let failure = app.failure();
    let shot = Shot::new(app, flag_value(&args, "--screenshot"), demo.pixels);
    let result = shot.result.clone();
    let options = WindowOptions {
        app_name: "demo".into(),
        title: format!("vapors demo: {name}"),
        width: 640,
        height: 400,
    };
    run(options, shot).map_err(|e| e.to_string())?;
    if let Some(message) = failure.get() {
        return Err(message);
    }
    result.borrow_mut().take().unwrap_or(Ok(()))
}

fn flag_value(args: &[String], flag: &str) -> Option<PathBuf> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
}

/// The app, and on its second frame the screenshot and its checks.
struct Shot {
    app: ElementsApp,
    path: Option<PathBuf>,
    pixels: Option<&'static str>,
    result: Rc<RefCell<Option<Result<(), String>>>>,
}

impl Shot {
    fn new(app: ElementsApp, path: Option<PathBuf>, pixels: Option<&'static str>) -> Self {
        Self {
            app,
            path,
            pixels,
            result: Rc::default(),
        }
    }

    fn check(&self, frame: &Frame<'_>, path: &PathBuf) -> Result<(), String> {
        let rgba = vapors_renderer::read_rgba8(frame.device, frame.queue, frame.texture);
        let (width, height) = (frame.texture.width(), frame.texture.height());
        write_png(path, width, height, &rgba)?;
        println!(
            "screenshot written {} {width}x{height} adapter={}",
            path.display(),
            frame.adapter.get_info().name
        );
        let Some(text) = self.pixels else {
            return Ok(());
        };
        #[allow(clippy::cast_possible_truncation)] // a display scale
        let image = Image {
            width,
            height,
            scale: frame.scale_factor as f32,
            rgba: &rgba,
        };
        let found = failures(&parse("demo", text)?, &image);
        if found.is_empty() {
            return Ok(());
        }
        Err(format!(
            "{} fails the demo's pixel checks:\n  {}",
            path.display(),
            found.join("\n  ")
        ))
    }
}

impl App for Shot {
    fn event(&mut self, event: Event, platform: &Platform) {
        self.app.event(event, platform);
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        if self.app.frame(frame) == Next::Exit {
            return Next::Exit;
        }
        match &self.path {
            Some(path) if frame.index >= 1 => {
                *self.result.borrow_mut() = Some(self.check(frame, path));
                Next::Exit
            }
            _ => Next::Continue,
        }
    }
}

fn write_png(path: &PathBuf, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    writer
        .write_image_data(rgba)
        .map_err(|e| format!("{}: {e}", path.display()))
}

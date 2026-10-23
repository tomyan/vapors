//! A demo in a window:
//!
//! ```text
//! cargo run -p vapors-app --example demo -- hello
//! cargo xtask screenshot demo target/hello.png -- hello
//! ```
//!
//! With `--screenshot <png>` (which `cargo xtask screenshot` adds) it
//! makes the demo's `.pixels` clicks after the first frame, saves the
//! second, checks it against the `.pixels`, and exits 1 when a check
//! fails or the document was refused.

use std::cell::RefCell;
use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_elements::Document;
use vapors_pixels::{Image, Spec, failures, parse};
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
    let document = vapors_demos::document(&demo)?;
    let app = ElementsApp::new(Rc::clone(&document));
    let failure = app.failure();
    let spec = parse(name, demo.pixels.unwrap_or_default())?;
    let shot = Shot::new(app, document, flag_value(&args, "--screenshot"), spec);
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

/// The app; after its first frame, the `.pixels` clicks; on the next,
/// the screenshot and its checks.
struct Shot {
    app: ElementsApp,
    document: Rc<RefCell<Document>>,
    path: Option<PathBuf>,
    spec: Spec,
    result: Rc<RefCell<Option<Result<(), String>>>>,
}

impl Shot {
    fn new(
        app: ElementsApp,
        document: Rc<RefCell<Document>>,
        path: Option<PathBuf>,
        spec: Spec,
    ) -> Self {
        Self {
            app,
            document,
            path,
            spec,
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
        #[allow(clippy::cast_possible_truncation)] // a display scale
        let image = Image {
            width,
            height,
            scale: frame.scale_factor as f32,
            rgba: &rgba,
        };
        let found = failures(&self.spec.checks, &image);
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

    /// The app's own answer, so a change that asks for no frame is not
    /// hidden: after the first frame the clicks are made and the app
    /// says whether they changed anything; the screenshot is the frame
    /// that follows (or the first, when there are no clicks).
    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        let next = self.app.frame(frame);
        let Some(path) = &self.path else { return next };
        if next == Next::Exit {
            return Next::Exit;
        }
        if frame.index == 0 && !self.spec.clicks.is_empty() {
            for point in &self.spec.clicks {
                vapors_demos::click(&self.document, *point);
            }
            return self.app.next_after_frame();
        }
        *self.result.borrow_mut() = Some(self.check(frame, path));
        Next::Exit
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

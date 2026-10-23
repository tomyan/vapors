//! The demos page. `?demo=<name>` picks what it shows: an elements demo
//! from `vapors-demos` (F2c), or `image`, the image app (F0: the
//! gradient fixture fitted to the canvas; a tap, click or `o` opens a
//! file), or `wake`, the waker's check (`wake.rs`). With no `?demo=` it
//! shows the index, which lists them. The
//! same binary runs natively, which is how `cargo clippy --workspace`
//! checks it; natively there is no query, so it shows the index.

mod wake;

use vapors_app::ElementsApp;
use vapors_canvas::{ImageApp, decode_png};
use vapors_platform::{WindowOptions, hide_page_controls, query_parameter, report_error, run};
use vapors_scene::Color;

const GRADIENT: &[u8] = include_bytes!("../../platform/examples/fixtures/gradient.png");

const BACKGROUND: Color = Color {
    r: 40.0 / 255.0,
    g: 40.0 / 255.0,
    b: 40.0 / 255.0,
    a: 1.0,
};

fn main() {
    if let Err(message) = start() {
        report_error(&message);
        #[cfg(not(target_family = "wasm"))]
        std::process::exit(1);
    }
}

fn start() -> Result<(), String> {
    let name = query_parameter("demo").unwrap_or_else(|| vapors_demos::INDEX.to_owned());
    let options = WindowOptions {
        app_name: "vapors".into(),
        title: format!("vapors demo: {name}"),
        width: 640,
        height: 400,
    };
    if name == "image" {
        let image = decode_png(GRADIENT).map_err(|message| format!("gradient.png: {message}"))?;
        let app = ImageApp::showing("gradient.png", image, BACKGROUND);
        return run(options, app).map_err(|e| e.to_string());
    }
    if name == "wake" {
        return run(options, wake::WakeApp).map_err(|e| e.to_string());
    }
    let demo = vapors_demos::find(&name)?;
    // An elements demo draws its own controls; the page's would cover them.
    hide_page_controls();
    let app = ElementsApp::new(vapors_demos::document(&demo)?);
    run(options, app).map_err(|e| e.to_string())
}

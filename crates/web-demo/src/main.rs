//! The image app on a page. Shows the gradient fixture fitted to the
//! canvas; a tap, click or `o` opens a file. The same binary runs
//! natively, which is how `cargo clippy --workspace` checks it.

use vapors_canvas::{ImageApp, decode_png};
use vapors_platform::{WindowOptions, report_error, run};
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
    let image = decode_png(GRADIENT).map_err(|message| format!("gradient.png: {message}"))?;
    let options = WindowOptions {
        app_name: "vapors".into(),
        title: "vapors web demo".into(),
        width: 640,
        height: 400,
    };
    run(options, ImageApp::showing(image, BACKGROUND)).map_err(|e| e.to_string())
}

//! Launch headless Chromium once before the gpu tier: the first launch on
//! a fresh runner is slower than the screenshot deadline allows (see
//! `COLD_START` in `comparison/chromium_warm.rs`). Exits 1, naming the
//! failure, when the browser is missing or the launch fails.

use std::process::ExitCode;

use vapors_text::comparison::Chromium;

fn main() -> ExitCode {
    let work = std::env::temp_dir().join("vapors-warm-chromium");
    let warmed = std::fs::create_dir_all(&work)
        .map_err(|e| format!("{}: {e}", work.display()))
        .and_then(|()| Chromium::find().map_err(|e| e.to_string()))
        .and_then(|chrome| {
            let took = chrome.warm(&work).map_err(|e| e.to_string())?;
            Ok((
                format!("{} ({})", chrome.binary().display(), chrome.version()),
                took,
            ))
        });
    match warmed {
        Ok((browser, took)) => {
            println!("ok   warm     {browser} in {:.1} s", took.as_secs_f64());
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("FAIL warm_chromium: {message}");
            eprintln!("     reproduce: cargo run -p vapors-text --example warm_chromium");
            ExitCode::FAILURE
        }
    }
}

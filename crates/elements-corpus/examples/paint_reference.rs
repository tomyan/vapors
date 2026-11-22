//! Captures a paint fixture's saved Chromium reference with the pinned
//! Chromium (saved references R1, `docs/plans/saved-chromium-references.md`):
//! `cargo xtask reference <fixture> <scale>` runs this. It writes the PNG
//! and its manifest under `fixtures/references/`, to be looked at and
//! committed.

use std::process::ExitCode;

use vapors_elements_corpus::paint_reference::capture;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (name, scale) = match args.as_slice() {
        [name, scale] => (name, scale.parse::<u32>()),
        _ => return usage(),
    };
    let Ok(scale) = scale else { return usage() };
    match capture(name, scale) {
        Ok(png) => {
            println!(
                "captured {}; look at it, then commit it with its manifest",
                png.display()
            );
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("paint_reference {name} {scale}: {why}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> ExitCode {
    eprintln!(
        "usage: cargo xtask reference <fixture> <scale>, as `cargo xtask reference text_line_box_paint 2`"
    );
    ExitCode::FAILURE
}

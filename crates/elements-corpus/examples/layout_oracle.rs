//! Writes Chromium's boxes for every layout fixture next to it:
//!
//! ```text
//! cargo run -p vapors-elements-corpus --example layout_oracle            # write fixtures/*.boxes.json
//! cargo run -p vapors-elements-corpus --example layout_oracle -- --check # fail on any difference
//! ```
//!
//! `--check` is what CI runs: a fixture edited without rerunning the
//! oracle, or a Chromium that lays a fixture out differently, fails it.

use std::process::ExitCode;

use vapors_elements_corpus::oracle::{
    boxes, fixture_names, fixtures_dir, to_text, write_fonts, write_images,
};
use vapors_text::comparison::Chromium;

fn main() -> ExitCode {
    let check = std::env::args().any(|arg| arg == "--check");
    match run(check) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("FAIL layout_oracle: {message}");
            eprintln!(
                "     reproduce: cargo run -p vapors-elements-corpus --example layout_oracle{}",
                if check { " -- --check" } else { "" }
            );
            ExitCode::FAILURE
        }
    }
}

fn run(check: bool) -> Result<(), String> {
    let chromium = Chromium::find().map_err(|e| e.to_string())?;
    println!("chromium: {}", chromium.version());
    let work = std::env::temp_dir().join(format!("vapors-layout-oracle-{}", std::process::id()));
    std::fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    write_fonts(&work)?;
    write_images(&work)?;
    let mut differ = Vec::new();
    for name in fixture_names()? {
        let text = to_text(&boxes(&chromium, &work, &name)?);
        let path = fixtures_dir().join(format!("{name}.boxes.json"));
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        if check && current != text {
            differ.push(name);
        } else if !check {
            std::fs::write(&path, &text).map_err(|e| format!("{}: {e}", path.display()))?;
            println!("wrote {}", path.display());
        }
    }
    let _ = std::fs::remove_dir_all(&work);
    if differ.is_empty() {
        return Ok(());
    }
    Err(format!(
        "Chromium's boxes differ from the committed ones for {}; rerun without --check, review the diff, and commit it",
        differ.join(", ")
    ))
}

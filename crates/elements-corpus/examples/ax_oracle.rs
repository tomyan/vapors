//! Writes Chromium's accessibility tree for every layout fixture next to
//! it, read over the `DevTools` protocol (docs/design/11-testing-oracles.md,
//! Q1):
//!
//! ```text
//! cargo run -p vapors-elements-corpus --example ax_oracle            # write fixtures/*.ax.json
//! cargo run -p vapors-elements-corpus --example ax_oracle -- --check # fail on any difference
//! ```
//!
//! `--check` is what CI runs: a fixture edited without rerunning the
//! oracle, or a Chromium whose tree differs, fails it.

use std::process::ExitCode;

use vapors_elements_corpus::ax::{start, tree};
use vapors_elements_corpus::oracle::{fixture_names, fixtures_dir, write_fonts};
use vapors_text::comparison::Chromium;

fn main() -> ExitCode {
    let check = std::env::args().any(|arg| arg == "--check");
    match run(check) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("FAIL ax_oracle: {message}");
            eprintln!(
                "     reproduce: cargo run -p vapors-elements-corpus --example ax_oracle{}",
                if check { " -- --check" } else { "" }
            );
            ExitCode::FAILURE
        }
    }
}

fn run(check: bool) -> Result<(), String> {
    let chromium = Chromium::find().map_err(|e| e.to_string())?;
    println!("chromium: {}", chromium.version());
    let work = std::env::temp_dir().join(format!("vapors-ax-oracle-{}", std::process::id()));
    std::fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    write_fonts(&work)?;
    let (_live, mut session) = start(&chromium, &work)?;
    let mut differ = Vec::new();
    for name in fixture_names()? {
        let tree = tree(&mut session, &work, &name)?;
        let text = serde_json::to_string_pretty(&tree).map_err(|e| e.to_string())? + "\n";
        let path = fixtures_dir().join(format!("{name}.ax.json"));
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
        "Chromium's accessibility trees differ from the committed ones for {}; rerun without --check, review the diff, and commit it",
        differ.join(", ")
    ))
}

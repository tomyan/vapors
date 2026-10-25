//! Plays each fixture's `.events` script in Chromium over the `DevTools`
//! protocol and writes its answers next to it
//! (docs/design/11-testing-oracles.md, Q1); every expectation must hold:
//!
//! ```text
//! cargo run -p vapors-elements-corpus --example events_oracle            # write fixtures/*.events.json
//! cargo run -p vapors-elements-corpus --example events_oracle -- --check # fail on any difference
//! ```
//!
//! `--check` is what CI runs: a fixture edited without rerunning the
//! oracle, or a Chromium whose answers differ, fails it, naming the first
//! line that differs. Without `--check` every answer is written, or none
//! when one fails. An `.events` or `*.events.json` with no fixture is refused.

use std::path::PathBuf;
use std::process::ExitCode;

use vapors_elements_corpus::ax::start;
use vapors_elements_corpus::events_oracle::{answers, scripted};
use vapors_elements_corpus::oracle::{fixtures_dir, write_fonts};
use vapors_elements_corpus::oracle_files::{Committed, Work, compare, refuse_orphans};
use vapors_text::comparison::Chromium;

fn main() -> ExitCode {
    let check = std::env::args().any(|arg| arg == "--check");
    match run(check) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("FAIL events_oracle: {message}");
            let flag = if check { " -- --check" } else { "" };
            eprintln!(
                "     reproduce: cargo run -p vapors-elements-corpus --example events_oracle{flag}"
            );
            ExitCode::FAILURE
        }
    }
}

fn run(check: bool) -> Result<(), String> {
    refuse_orphans(".events.json")?;
    let trees = trees()?;
    if check {
        checked(&trees)
    } else {
        written(&trees)
    }
}

/// Every scripted fixture's answers as committed text, each beside its path.
fn trees() -> Result<Vec<(String, PathBuf, String)>, String> {
    let chromium = Chromium::find().map_err(|e| e.to_string())?;
    println!("chromium: {}", chromium.version());
    let work = Work::new("vapors-events-oracle")?;
    write_fonts(work.path())?;
    let (_live, mut session) = start(&chromium, work.path())?;
    let mut trees = Vec::new();
    for name in scripted()? {
        let answers = answers(&mut session, work.path(), &name)?;
        let text = serde_json::to_string_pretty(&answers).map_err(|e| e.to_string())? + "\n";
        trees.push((
            name.clone(),
            fixtures_dir().join(format!("{name}.events.json")),
            text,
        ));
    }
    Ok(trees)
}

fn checked(trees: &[(String, PathBuf, String)]) -> Result<(), String> {
    let mut reports = Vec::new();
    for (name, path, text) in trees {
        let committed = compare(path, text)?;
        if committed != Committed::Same {
            reports.push(committed.report(name));
        }
    }
    if reports.is_empty() {
        return Ok(());
    }
    Err(format!(
        "Chromium's answers to the .events scripts differ from the committed ones:\n{}",
        reports.join("\n")
    ))
}

fn written(trees: &[(String, PathBuf, String)]) -> Result<(), String> {
    for (_, path, text) in trees {
        std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

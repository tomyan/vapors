//! Captures saved Chromium references for the paint tests
//! (`docs/plans/saved-chromium-references.md`), run by
//! `cargo xtask reference all` or `cargo xtask reference <fixture>
//! <scale> [<state>]`. It writes PNGs under `fixtures/references/`, to
//! be looked at before they are committed.

use std::process::ExitCode;

use vapors_elements_corpus::paint_reference::{Reference, capture, every_reference};

const USAGE: &str = "usage: cargo xtask reference all | <fixture> <scale> [<state>], \
                     as `cargo xtask reference text_line_box_paint 2`";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let references = match references(&args) {
        Ok(references) => references,
        Err(why) => {
            eprintln!("paint_reference: {why}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    for reference in &references {
        match capture(reference) {
            Ok(png) => println!("captured {}", png.display()),
            Err(why) => {
                eprintln!("{}: {why}", reference.command());
                return ExitCode::FAILURE;
            }
        }
    }
    println!(
        "look at each before committing it: a mismatch is fixed in vapors unless the reference is wrong"
    );
    ExitCode::SUCCESS
}

fn references(args: &[String]) -> Result<Vec<Reference>, String> {
    match args {
        [all] if all == "all" => Ok(every_reference()),
        [fixture, scale] => one(fixture, scale, None),
        [fixture, scale, state] => one(fixture, scale, Some(state)),
        _ => Err(format!("{args:?} names no reference")),
    }
}

fn one(fixture: &str, scale: &str, state: Option<&String>) -> Result<Vec<Reference>, String> {
    let scale = scale
        .parse::<u32>()
        .map_err(|e| format!("scale {scale:?}: {e}"))?;
    Ok(vec![Reference::of(
        fixture,
        state.map(String::as_str),
        scale,
    )?])
}

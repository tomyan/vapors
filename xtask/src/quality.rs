//! `cargo xtask quality`: the quality signals, advisory and never
//! failing on what they find. The hard checks sit at the tail of each
//! distribution (clippy's `too_many_lines` and `cognitive_complexity`
//! at clippy.toml's thresholds, `[files] max_lines`); the band between
//! the guideline (`[quality]` in .config/factory.toml) and the hard
//! limit is for review, and this prints its worst offenders so a
//! reviewer sees them without counting. `--distributions` prints what
//! the thresholds were set from: median, p90, p99 and max with the ten
//! worst, code and tests apart
//! (docs/decisions/2026-11-13-quality-thresholds.md).
//!
//! The command fails only when it cannot measure: a file that does not
//! parse, or a threshold it cannot read.

use std::path::Path;
use std::process::ExitCode;

use crate::config::Config;
use crate::files::Rule;
use crate::quality_collect::{Collected, Split, collect};
use crate::quality_stats::{band_lines, summary_lines};
use crate::quality_thresholds::{Guideline, Hard};

/// What to print.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The advisory signals.
    Signals,
    /// The distributions behind the thresholds.
    Distributions,
    /// Every sample, for checking against clippy.
    List,
}

impl Mode {
    /// From the command's flags; clap keeps them exclusive.
    pub fn from_flags(distributions: bool, list: bool) -> Self {
        match (distributions, list) {
            (true, _) => Self::Distributions,
            (_, true) => Self::List,
            _ => Self::Signals,
        }
    }
}

/// Run from the workspace root with the command's flags.
pub fn run(distributions: bool, list: bool) -> ExitCode {
    match lines(Path::new("."), Mode::from_flags(distributions, list)) {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(message) => {
            println!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn lines(root: &Path, mode: Mode) -> Result<Vec<String>, String> {
    let config = Config::load_from(root).map_err(|e| e.to_string())?;
    let collected = collect(root, &Rule::from_config(&config)?.generated)?;
    match mode {
        Mode::Distributions => return Ok(distribution_lines(&collected)),
        Mode::List => return Ok(list_lines(&collected)),
        Mode::Signals => {}
    }
    let guideline = Guideline::from_config(&config)?;
    let hard = Hard::read(root, &config)?;
    Ok(signal_lines(&collected, &guideline, &hard))
}

/// The advisory section: four signals, each a count and its worst.
pub fn signal_lines(collected: &Collected, guideline: &Guideline, hard: &Hard) -> Vec<String> {
    let mut lines = vec![
        "quality signals (advisory; the band from the guideline to the hard limit; `cargo xtask quality`)"
            .to_owned(),
    ];
    let over = |what: &str, n: usize, unit: &str| format!("{what} over {n} {unit}");
    lines.extend(band_lines(
        &over("functions", guideline.function_lines, "code lines"),
        &collected.function_lines.all(),
        guideline.function_lines,
        Some(hard.function_lines),
        3,
    ));
    lines.extend(band_lines(
        &format!("complexity over {}", guideline.complexity),
        &collected.complexity.all(),
        guideline.complexity,
        Some(hard.complexity),
        3,
    ));
    lines.extend(band_lines(
        &over("files", guideline.file_lines, "lines"),
        &collected.files.all(),
        guideline.file_lines,
        Some(hard.file_lines),
        2,
    ));
    lines.extend(band_lines(
        &format!("field chains of {}+ hops", guideline.chain_hops),
        &collected.chains,
        guideline.chain_hops - 1,
        None,
        2,
    ));
    lines
}

/// The measurement behind the thresholds.
pub fn distribution_lines(collected: &Collected) -> Vec<String> {
    let mut lines = Vec::new();
    for (name, split) in [
        ("file lines", &collected.files),
        (
            "function code lines (clippy's too_many_lines count)",
            &collected.function_lines,
        ),
        (
            "cognitive complexity (clippy's count)",
            &collected.complexity,
        ),
    ] {
        lines.extend(split_lines(name, split));
    }
    lines.extend(summary_lines(
        "field chain hops, code",
        &collected.chains,
        10,
    ));
    lines
}

/// Every sample as `<metric>\t<value>\t<where>`.
pub fn list_lines(collected: &Collected) -> Vec<String> {
    let mut lines = Vec::new();
    for (metric, samples) in [
        ("file_lines", collected.files.all()),
        ("function_lines", collected.function_lines.all()),
        ("complexity", collected.complexity.all()),
        ("chain_hops", collected.chains.clone()),
    ] {
        lines.extend(
            samples
                .iter()
                .map(|s| format!("{metric}\t{}\t{}", s.value, s.label)),
        );
    }
    lines
}

fn split_lines(name: &str, split: &Split) -> Vec<String> {
    let mut lines = summary_lines(&format!("{name}, code"), &split.code, 10);
    lines.extend(summary_lines(&format!("{name}, tests"), &split.tests, 10));
    lines
}

#[cfg(test)]
#[path = "quality_tests.rs"]
mod tests;

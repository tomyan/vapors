//! `cargo xtask quality`: the quality signals, advisory and never
//! failing on what they find. The hard checks sit at the tail of each
//! distribution (clippy's `too_many_lines` and `cognitive_complexity`
//! at clippy.toml's thresholds, `[files] max_lines`, all in `lint` and
//! `ci`); the band between the guideline (`[quality]` in
//! .config/factory.toml) and the hard limit is for review, and this
//! prints the band's worst so a reviewer sees them without counting.
//! What is over a hard limit is clippy's or check-files' business,
//! allowed at its site or failing, and is not listed. The nightly run
//! (`.github/workflows/nightly.yml`) prints it; it is not a step of
//! `lint` or `ci`, which stay checks that fail loud
//! (docs/decisions/2026-11-13-quality-thresholds.md).
//!
//! `--distributions` prints the measurement the thresholds were set
//! from (`quality_distributions.rs`) and fails when xtask's counts have
//! drifted from clippy's; `--list` prints every sample. The command
//! fails when it cannot measure: a file that does not parse, a
//! threshold it cannot read.

use std::path::Path;
use std::process::ExitCode;

use crate::config::Config;
use crate::files::Rule;
use crate::quality_clippy::collect_clippy;
use crate::quality_collect::{Collected, collect};
use crate::quality_distributions::{Report, distribution_report};
use crate::quality_join::join;
use crate::quality_stats::band_lines;
use crate::quality_thresholds::{Guideline, Hard};

/// What to print.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The advisory signals.
    Signals,
    /// The distributions behind the thresholds, with clippy's counts.
    Distributions,
    /// Every sample.
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
    let report = match report(Path::new("."), Mode::from_flags(distributions, list)) {
        Ok(report) => report,
        Err(message) => Report {
            lines: Vec::new(),
            refusal: Some(message),
        },
    };
    for line in &report.lines {
        println!("{line}");
    }
    match report.refusal {
        None => ExitCode::SUCCESS,
        Some(message) => {
            println!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn report(root: &Path, mode: Mode) -> Result<Report, String> {
    let config = Config::load_from(root).map_err(|e| e.to_string())?;
    let collected = collect(root, &Rule::from_config(&config)?.generated)?;
    let lines = match mode {
        Mode::Distributions => {
            let joined = join(&collected, &collect_clippy(root)?);
            return Ok(distribution_report(&collected, &joined));
        }
        Mode::List => list_lines(&collected),
        Mode::Signals => {
            let guideline = Guideline::from_config(&config)?;
            let hard = Hard::read(root, &config)?;
            signal_lines(&collected, &guideline, &hard)
        }
    };
    Ok(Report {
        lines,
        refusal: None,
    })
}

/// The advisory section: four signals, each the band's count and its worst.
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

#[cfg(test)]
#[path = "quality_tests.rs"]
mod tests;

//! clippy's own counts for every function, for `cargo xtask quality
//! --distributions`: clippy is run with both thresholds at 1 (a
//! `clippy.toml` in a temporary `CLIPPY_CONF_DIR`), its JSON diagnostics
//! read, and each count keyed by file and the function's `fn` line,
//! where both lints point. `quality_join.rs` joins them to xtask's
//! functions; the agreement is what holds xtask's band to clippy's hard
//! limit, and `--distributions` fails under `FLOOR_*`: lines agreed on
//! 99.96% of main's functions on 2026-11-13 (the rest `async fn`s),
//! complexity on 97.5% (third-party macros with branches, returns inside
//! closures, aliases of `Result`), as
//! `docs/decisions/2026-11-13-quality-thresholds.md` records. The run
//! writes its `clippy.toml` under `target/quality-clippy-conf` and
//! re-lints the workspace crates, so the next ordinary clippy run
//! re-lints them too.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// Agreement on lines the command refuses under, in tenths of a percent.
pub const FLOOR_LINES_PER_MILLE: usize = 999;
/// Agreement on complexity the command refuses under, in tenths of a percent.
pub const FLOOR_COMPLEXITY_PER_MILLE: usize = 970;

/// clippy's counts by `(path, line)`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ClippyCounts {
    /// `too_many_lines`, at the `fn` line.
    pub lines: BTreeMap<(String, usize), usize>,
    /// `cognitive_complexity`, at the `fn` line too.
    pub complexity: BTreeMap<(String, usize), usize>,
}

/// Run clippy under `root` with both thresholds at 1 and read its counts.
///
/// # Errors
///
/// clippy failing (its last lines), or a diagnostic of a shape not read.
pub fn collect_clippy(root: &Path) -> Result<ClippyCounts, String> {
    let conf = root.join("target").join("quality-clippy-conf");
    std::fs::create_dir_all(&conf).map_err(|e| format!("{}: {e}", conf.display()))?;
    std::fs::write(
        conf.join("clippy.toml"),
        "too-many-lines-threshold = 1\ncognitive-complexity-threshold = 1\n",
    )
    .map_err(|e| format!("{}: {e}", conf.display()))?;
    let output = Command::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--no-deps",
            "--message-format=json",
        ])
        .args([
            "--",
            "-W",
            "clippy::cognitive_complexity",
            "-W",
            "clippy::too_many_lines",
        ])
        .env("CLIPPY_CONF_DIR", &conf)
        .current_dir(root)
        .output()
        .map_err(|e| {
            format!("cargo clippy: {e}; fix: run from a workspace with the toolchain installed")
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: Vec<&str> = stderr
            .lines()
            .rev()
            .take(15)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        return Err(format!(
            "cargo clippy failed ({}):\n{}",
            output.status,
            tail.join("\n")
        ));
    }
    parse_counts(&String::from_utf8_lossy(&output.stdout))
}

/// The counts in cargo's JSON message stream.
///
/// # Errors
///
/// A `too_many_lines` or `cognitive_complexity` diagnostic without a
/// primary span or a `(N/1)` in its message, quoted.
pub fn parse_counts(stream: &str) -> Result<ClippyCounts, String> {
    let mut counts = ClippyCounts::default();
    for line in stream.lines() {
        let Ok(message) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if message["reason"] != "compiler-message" {
            continue;
        }
        let diagnostic = &message["message"];
        let table = match diagnostic["code"]["code"].as_str() {
            Some("clippy::too_many_lines") => &mut counts.lines,
            Some("clippy::cognitive_complexity") => &mut counts.complexity,
            _ => continue,
        };
        let (key, value) = read_diagnostic(diagnostic)?;
        table.insert(key, value);
    }
    Ok(counts)
}

fn read_diagnostic(diagnostic: &serde_json::Value) -> Result<((String, usize), usize), String> {
    let spans = diagnostic["spans"].as_array().cloned().unwrap_or_default();
    let primary = spans
        .iter()
        .find(|s| s["is_primary"] == true)
        .ok_or_else(|| format!("clippy diagnostic without a primary span: {diagnostic}"))?;
    let path = primary["file_name"].as_str().unwrap_or_default().to_owned();
    let line = primary["line_start"].as_u64().unwrap_or_default();
    let text = diagnostic["message"].as_str().unwrap_or_default();
    let value = text
        .rsplit_once('(')
        .and_then(|(_, rest)| rest.split_once('/'))
        .and_then(|(n, _)| n.parse::<usize>().ok())
        .ok_or_else(|| format!("clippy message without `(N/1)`: {text}"))?;
    Ok(((path, usize::try_from(line).unwrap_or_default()), value))
}

#[cfg(test)]
#[path = "quality_clippy_tests.rs"]
mod tests;

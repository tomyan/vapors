//! The file-length rule as a check: every source file under the
//! workspace is at most `[files] max_lines` long, unless it is named
//! under `[files.generated]` or allowed under `[files.allow]` with a
//! reason. The failure names each file with its line count and the fix;
//! an allowance naming no file, or a file now within the limit, is
//! refused as stale. Allowed files and their reasons are printed when
//! `cargo xtask check-files` runs by itself or `lint` runs with
//! `--verbose`; the `files` step shows only the refusals.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use crate::config::Config;
use crate::files_walk::{count_lines, workspace_files};

/// The rule says about 200. 220 leaves room for a module doc comment
/// and a test-module mount without forcing a split that would only
/// move lines; a file past it has a second responsibility to give away.
pub const DEFAULT_MAX_LINES: usize = 220;

/// The rule as configured.
#[derive(Debug, PartialEq, Eq)]
pub struct Rule {
    /// Lines a file may have.
    pub max_lines: usize,
    /// Files not written by hand, by path from the workspace root.
    pub generated: BTreeMap<String, String>,
    /// Files allowed over the limit, each with its reason.
    pub allow: BTreeMap<String, String>,
}

/// One file over the limit, or allowed over it.
#[derive(Debug, PartialEq, Eq)]
pub struct Finding {
    /// Path from the workspace root.
    pub path: String,
    /// Its line count.
    pub lines: usize,
    /// The allow reason, when allowed.
    pub allowed: Option<String>,
}

impl Rule {
    /// Read `[files]`, `[files.generated]` and `[files.allow]`.
    ///
    /// # Errors
    ///
    /// A message when `max_lines` is not a number.
    pub fn from_config(config: &Config) -> Result<Self, String> {
        let max_lines = match config.get("files.max_lines") {
            None => DEFAULT_MAX_LINES,
            Some(text) => text.parse().map_err(|_| {
                format!(".config/factory.toml: [files] max_lines = \"{text}\" is not a number")
            })?,
        };
        Ok(Self {
            max_lines,
            generated: config.entries_under("files.generated."),
            allow: config.entries_under("files.allow."),
        })
    }
}

/// Run the check from the workspace root; `cargo xtask check-files`.
pub fn run() -> ExitCode {
    match check() {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(message) => {
            // Every refusal starts with `error:` so the reporter shows it.
            for line in message.lines() {
                if line.starts_with("error:") || line.starts_with("allowed") {
                    println!("{line}");
                } else {
                    println!("error: {line}");
                }
            }
            ExitCode::FAILURE
        }
    }
}

fn check() -> Result<Vec<String>, String> {
    let rule = Rule::from_config(&Config::load().map_err(|e| e.to_string())?)?;
    let counted = workspace_files(Path::new("."))?
        .into_iter()
        .map(|path| {
            let lines = count_lines(&path)?;
            Ok((path.display().to_string(), lines))
        })
        .collect::<Result<Vec<_>, String>>()?;
    check_allowances(&counted, &rule)?;
    report(&over_limit(&counted, &rule), &rule)
}

/// Every `[files.allow]` and `[files.generated]` entry names a counted
/// file, and every allowed file is still over the limit.
///
/// # Errors
///
/// One `error:` line per stale entry, naming it and the fix.
pub fn check_allowances(counted: &[(String, usize)], rule: &Rule) -> Result<(), String> {
    let mut stale = Vec::new();
    for (table, entries) in [("allow", &rule.allow), ("generated", &rule.generated)] {
        for path in entries.keys() {
            match counted.iter().find(|(p, _)| p == path) {
                None => stale.push(format!(
                    "error: [files.{table}] `{path}` names no file; stale allowance; remove it"
                )),
                Some((_, lines)) if table == "allow" && *lines <= rule.max_lines => {
                    stale.push(format!(
                        "error: [files.allow] `{path}` is {lines} lines, within {}; stale allowance; remove it",
                        rule.max_lines
                    ));
                }
                Some(_) => {}
            }
        }
    }
    if stale.is_empty() {
        Ok(())
    } else {
        Err(stale.join("\n"))
    }
}

/// The files over `rule.max_lines`, allowed or not; generated ones are
/// left out.
pub fn over_limit(counted: &[(String, usize)], rule: &Rule) -> Vec<Finding> {
    counted
        .iter()
        .filter(|(path, lines)| *lines > rule.max_lines && !rule.generated.contains_key(path))
        .map(|(path, lines)| Finding {
            path: path.clone(),
            lines: *lines,
            allowed: rule.allow.get(path).cloned(),
        })
        .collect()
}

/// The lines to print: one per allowed file with its reason; an error
/// naming every file over the limit with the fix. Each starts with
/// `error:` so the reporter shows it without `--verbose`.
pub fn report(findings: &[Finding], rule: &Rule) -> Result<Vec<String>, String> {
    let allowed: Vec<String> = findings
        .iter()
        .filter_map(|f| {
            f.allowed
                .as_ref()
                .map(|reason| format!("allowed  {} ({} lines): {reason}", f.path, f.lines))
        })
        .collect();
    let over: Vec<String> = findings
        .iter()
        .filter(|f| f.allowed.is_none())
        .map(|f| {
            format!(
                "error: {}: {} lines, over {}; fix: split it, or allow it in .config/factory.toml under [files.allow] as `{} = \"<reason>\"`",
                f.path, f.lines, rule.max_lines, f.path
            )
        })
        .collect();
    if over.is_empty() {
        Ok(allowed)
    } else {
        Err([allowed, over].concat().join("\n"))
    }
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;

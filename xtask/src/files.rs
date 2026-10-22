//! The file-length rule as a check: every source file under the
//! workspace is at most `[files] max_lines` long, unless it is named
//! under `[files.generated]` or allowed under `[files.allow]` with a
//! reason, which is printed. The failure names each file with its line
//! count and the fix.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::config::Config;

/// The rule says about 200. 220 leaves room for a module doc comment
/// and a test-module mount without forcing a split that would only
/// move lines; a file past it has a second responsibility to give away.
pub const DEFAULT_MAX_LINES: usize = 220;

/// Extensions the rule covers.
const EXTENSIONS: [&str; 5] = ["rs", "wgsl", "html", "js", "toml"];

/// Directories never walked: build output, git, dependencies, fonts.
const SKIPPED_DIRS: [&str; 4] = ["target", ".git", "node_modules", "fonts"];

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
            println!("{message}");
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
    report(&over_limit(&counted, &rule), &rule)
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

/// Every covered file under `root`, as `crates/...` paths, sorted.
///
/// # Errors
///
/// A message naming a directory that could not be read.
pub fn workspace_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    walk(root, Path::new(""), &mut found)?;
    found.sort();
    Ok(found)
}

fn walk(root: &Path, relative: &Path, found: &mut Vec<PathBuf>) -> Result<(), String> {
    let dir = root.join(relative);
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let path = relative.join(&name);
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() && !SKIPPED_DIRS.contains(&name.to_string_lossy().as_ref()) {
            walk(root, &path, found)?;
        } else if kind.is_file() && covered(&path) {
            found.push(path);
        }
    }
    Ok(())
}

fn covered(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e))
}

/// Lines in `path`, as `wc -l` counts them.
///
/// # Errors
///
/// A message naming the file when it cannot be read.
#[allow(clippy::naive_bytecount)] // source files are small; a crate for this is not worth a dependency
pub fn count_lines(path: &Path) -> Result<usize, String> {
    let text = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(text.iter().filter(|&&b| b == b'\n').count())
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;

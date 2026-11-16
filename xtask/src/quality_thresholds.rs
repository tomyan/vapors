//! Where the quality numbers live, read so the signals name them:
//! the guidelines under `[quality]` in .config/factory.toml, the hard
//! limits in clippy.toml (`too-many-lines-threshold`,
//! `cognitive-complexity-threshold`) and `[files] max_lines`. A missing
//! or unreadable number is refused naming the file and the key: the
//! band has no meaning without both ends.

use std::path::Path;

use crate::config::Config;
use crate::files::Rule;

/// The guideline values: a function or file over one is in the band
/// the signals print; a chain of this many hops reaches too far.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guideline {
    /// Lines of code in a function body.
    pub function_lines: usize,
    /// Cognitive complexity.
    pub complexity: usize,
    /// Lines in a file.
    pub file_lines: usize,
    /// Field hops in a chain.
    pub chain_hops: usize,
}

/// The hard limits the checks fail at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hard {
    /// clippy's `too-many-lines-threshold`.
    pub function_lines: usize,
    /// clippy's `cognitive-complexity-threshold`.
    pub complexity: usize,
    /// `[files] max_lines`.
    pub file_lines: usize,
}

impl Guideline {
    /// Read `[quality]`.
    ///
    /// # Errors
    ///
    /// A message naming the key that is missing or not a number.
    pub fn from_config(config: &Config) -> Result<Self, String> {
        Ok(Self {
            function_lines: number(config, "quality.function_lines")?,
            complexity: number(config, "quality.complexity")?,
            file_lines: number(config, "quality.file_lines")?,
            chain_hops: number(config, "quality.chain_hops")?,
        })
    }
}

impl Hard {
    /// Read clippy.toml under `root` and `[files] max_lines`.
    ///
    /// # Errors
    ///
    /// A message naming the file and key that is missing or not a number.
    pub fn read(root: &Path, config: &Config) -> Result<Self, String> {
        let clippy = root.join("clippy.toml");
        let text = std::fs::read_to_string(&clippy).map_err(|e| {
            format!(
                "{}: {e}; fix: add clippy.toml with the thresholds",
                clippy.display()
            )
        })?;
        Ok(Self {
            function_lines: clippy_number(&text, "too-many-lines-threshold")?,
            complexity: clippy_number(&text, "cognitive-complexity-threshold")?,
            file_lines: Rule::from_config(config)?.max_lines,
        })
    }
}

fn number(config: &Config, key: &str) -> Result<usize, String> {
    let text = config.require(key).map_err(|e| e.to_string())?;
    text.parse().map_err(|_| {
        format!(".config/factory.toml: {key} = \"{text}\" is not a number; fix: set it to one")
    })
}

/// `key = <number>` in clippy.toml's text, comments and blank lines skipped.
fn clippy_number(text: &str, key: &str) -> Result<usize, String> {
    let value = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.trim_start().strip_prefix('='))
        })
        .ok_or_else(|| {
            format!("clippy.toml: no `{key} = <number>`; fix: set it from the measurement")
        })?;
    value.trim().parse().map_err(|_| {
        format!(
            "clippy.toml: `{key} = {}` is not a number; fix: set it to one",
            value.trim()
        )
    })
}

#[cfg(test)]
#[path = "quality_thresholds_tests.rs"]
mod tests;

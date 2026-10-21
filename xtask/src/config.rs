//! `.config/factory.toml`: the per-repository settings the shared xtask
//! code reads. Flat tables of `key = "value"`; anything else is refused
//! with its line number.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;

const FILE: &str = ".config/factory.toml";

/// Settings as `table.key` to value.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Config {
    values: BTreeMap<String, String>,
}

/// Why the settings could not be read.
#[derive(Debug, PartialEq, Eq)]
pub struct ConfigError {
    /// 1-based line, or 0 when the file could not be opened.
    pub line: usize,
    /// What was wrong.
    pub reason: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{FILE}:{}: {}", self.line, self.reason)
    }
}

impl Config {
    /// Read the settings file from the workspace root.
    pub fn load() -> Result<Self, ConfigError> {
        let text = fs::read_to_string(FILE).map_err(|e| ConfigError {
            line: 0,
            reason: format!("could not read it: {e}; run from the workspace root"),
        })?;
        Self::parse(&text)
    }

    /// Parse the settings text.
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let mut table: Option<String> = None;
        let mut values = BTreeMap::new();
        for (index, raw) in text.lines().enumerate() {
            let line = raw.trim();
            let number = index + 1;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                table = Some(name.trim().to_owned());
                continue;
            }
            let Some(table) = &table else {
                return Err(ConfigError {
                    line: number,
                    reason: format!("entry before any [table] header: `{line}`"),
                });
            };
            let (key, value) = key_value(line, number)?;
            values.insert(format!("{table}.{key}"), value);
        }
        Ok(Self { values })
    }

    /// The value for `table.key`, or an error naming it and the file.
    pub fn require(&self, key: &str) -> Result<&str, ConfigError> {
        self.values
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| ConfigError {
                line: 0,
                reason: format!(
                    "missing `{key}`; add it as `[{}]` then `{} = \"...\"`",
                    table_of(key),
                    name_of(key)
                ),
            })
    }
}

fn table_of(key: &str) -> &str {
    key.split_once('.').map_or(key, |(t, _)| t)
}

fn name_of(key: &str) -> &str {
    key.split_once('.').map_or(key, |(_, n)| n)
}

fn key_value(line: &str, number: usize) -> Result<(String, String), ConfigError> {
    let refuse = |reason: &str| ConfigError {
        line: number,
        reason: format!("{reason}: `{line}`"),
    };
    let (key, rest) = line
        .split_once('=')
        .ok_or_else(|| refuse("expected `key = \"value\"`"))?;
    let value = rest
        .trim()
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .ok_or_else(|| refuse("value must be a double-quoted string"))?;
    if key.trim().is_empty() || value.is_empty() {
        return Err(refuse("key and value must both be non-empty"));
    }
    Ok((key.trim().to_owned(), value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_value_by_table_and_key() {
        // Given
        let text = "# settings\n[screenshot]\npackage = \"vapors-platform\"\n";

        // When
        let config = Config::parse(text).expect("valid");

        // Then
        assert_eq!(config.require("screenshot.package"), Ok("vapors-platform"));
    }

    #[test]
    fn a_missing_key_says_how_to_add_it() {
        // Given
        let config = Config::parse("[screenshot]\n").expect("valid");

        // When
        let error = config.require("screenshot.package").expect_err("missing");

        // Then
        assert!(error.reason.contains("[screenshot]"));
        assert!(error.reason.contains("package = "));
    }

    #[test]
    fn refuses_an_entry_before_a_header_naming_the_line() {
        // Given
        let text = "package = \"x\"\n";

        // When
        let error = Config::parse(text).expect_err("no header");

        // Then
        assert_eq!(error.line, 1);
    }
}

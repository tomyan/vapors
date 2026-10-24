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
    /// Read the settings file from the workspace root, which is the
    /// current directory.
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_from(std::path::Path::new("."))
    }

    /// Read the settings file under `root`, for a command that runs from
    /// elsewhere (trunk runs its hooks in the page's crate).
    pub fn load_from(root: &std::path::Path) -> Result<Self, ConfigError> {
        let path = root.join(FILE);
        let text = fs::read_to_string(&path).map_err(|e| ConfigError {
            line: 0,
            reason: format!(
                "could not read {}: {e}; run from the workspace root",
                path.display()
            ),
        })?;
        Self::parse(&text)
    }

    /// Parse the settings text.
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let mut table: Option<String> = None;
        let mut values = BTreeMap::new();
        let mut lines_of: BTreeMap<String, usize> = BTreeMap::new();
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
            let full = format!("{table}.{key}");
            if let Some(first) = lines_of.insert(full.clone(), number) {
                return Err(ConfigError {
                    line: number,
                    reason: format!("duplicate key `{full}`; first on line {first}"),
                });
            }
            values.insert(full, value);
        }
        Ok(Self { values })
    }

    /// The value for `table.key`, if set.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// Every `key = value` whose full key starts with `prefix` (such as
    /// `files.allow.`), keyed by the rest, so a table can hold a list of
    /// paths with a reason each.
    pub fn entries_under(&self, prefix: &str) -> BTreeMap<String, String> {
        self.values
            .iter()
            .filter_map(|(key, value)| {
                key.strip_prefix(prefix)
                    .map(|rest| (rest.to_owned(), value.clone()))
            })
            .collect()
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
    let (key, rest) = split_key(line).ok_or_else(|| refuse("expected `key = \"value\"`"))?;
    let value = rest
        .trim()
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .ok_or_else(|| refuse("value must be a double-quoted string"))?;
    let key = unquoted_key(key.trim()).ok_or_else(|| {
        refuse("key must be bare (no quotes or spaces) or a double-quoted string without escapes")
    })?;
    if key.is_empty() || value.is_empty() {
        return Err(refuse("key and value must both be non-empty"));
    }
    Ok((key, value.to_owned()))
}

/// `key` and the rest after its `=`: a quoted key ends at its closing
/// quote, so it may hold `=` (a gate setup line such as `:lang=zh`).
fn split_key(line: &str) -> Option<(&str, &str)> {
    let quote = line.chars().next().filter(|c| *c == '"' || *c == '\'');
    let quoted = quote.and_then(|quote| {
        let end = line[1..].find(quote)? + 2;
        let rest = line[end..].trim_start().strip_prefix('=')?;
        Some((&line[..end], rest))
    });
    // Not a quoted key followed by `=`: split at the first `=`, so a
    // malformed key is refused by name below.
    quoted.or_else(|| line.split_once('='))
}

/// A bare key as is; a double-quoted key without its quotes; a
/// single-quoted key (TOML's literal string, which may hold double
/// quotes) as written between them; `None` for anything else (a quote
/// inside, a backslash in a double-quoted key, a space in a bare key).
fn unquoted_key(key: &str) -> Option<String> {
    if let Some(inner) = key.strip_prefix('\'') {
        let inner = inner.strip_suffix('\'')?;
        return (!inner.contains('\'')).then(|| inner.to_owned());
    }
    if let Some(inner) = key.strip_prefix('"') {
        let inner = inner.strip_suffix('"')?;
        return (!inner.contains(['"', '\\'])).then(|| inner.to_owned());
    }
    (!key.contains(['"', ' ', '\t'])).then(|| key.to_owned())
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;

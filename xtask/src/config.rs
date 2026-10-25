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
    tables: BTreeMap<String, usize>,
    keys: Vec<Placed>,
}

/// A key as written: its table, its name and its line.
#[derive(Debug, PartialEq, Eq)]
pub struct Placed {
    /// The table it is under.
    pub table: String,
    /// Its name.
    pub key: String,
    /// Its 1-based line.
    pub line: usize,
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
    /// elsewhere (trunk runs its hooks in the page's crate). A table or key
    /// the factory does not read is refused (`config_schema`).
    pub fn load_from(root: &std::path::Path) -> Result<Self, ConfigError> {
        let path = root.join(FILE);
        let text = fs::read_to_string(&path).map_err(|e| ConfigError {
            line: 0,
            reason: format!(
                "could not read {}: {e}; run from the workspace root",
                path.display()
            ),
        })?;
        let config = Self::parse(&text)?;
        crate::config_schema::check_known(&config)?;
        Ok(config)
    }

    /// Parse the settings text.
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let mut table: Option<String> = None;
        let mut values = BTreeMap::new();
        let mut tables = BTreeMap::new();
        let mut keys = Vec::new();
        let mut lines_of: BTreeMap<String, usize> = BTreeMap::new();
        for (index, raw) in text.lines().enumerate() {
            let line = raw.trim();
            let number = index + 1;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                table = Some(name.trim().to_owned());
                tables.insert(name.trim().to_owned(), number);
                continue;
            }
            let Some(table) = &table else {
                return Err(ConfigError {
                    line: number,
                    reason: format!("entry before any [table] header: `{line}`"),
                });
            };
            let (key, value) = crate::config_line::key_value(line, number)?;
            let full = format!("{table}.{key}");
            if let Some(first) = lines_of.insert(full.clone(), number) {
                return Err(ConfigError {
                    line: number,
                    reason: format!("duplicate key `{full}`; first on line {first}"),
                });
            }
            keys.push(Placed {
                table: table.clone(),
                key,
                line: number,
            });
            values.insert(full, value);
        }
        Ok(Self {
            values,
            tables,
            keys,
        })
    }

    /// Every table header equal to `prefix` or under `prefix.`, empty
    /// ones included, sorted.
    pub fn tables_under(&self, prefix: &str) -> Vec<&str> {
        let under = format!("{prefix}.");
        self.tables
            .keys()
            .filter(|t| *t == prefix || t.starts_with(&under))
            .map(String::as_str)
            .collect()
    }

    /// Every table header and its line.
    pub fn table_lines(&self) -> impl Iterator<Item = (&str, usize)> {
        self.tables
            .iter()
            .map(|(table, line)| (table.as_str(), *line))
    }

    /// Every key, with its table and line.
    pub fn placed_keys(&self) -> &[Placed] {
        &self.keys
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

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;

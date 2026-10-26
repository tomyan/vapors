//! The tables and keys the factory reads. A table or key it does not
//! read is refused with its line, so a misspelling such as
//! `[layer.forbid]` cannot pass as a rule nobody checks.

use crate::config::{Config, ConfigError};

/// A table's keys: listed names, or any key (a list of paths, rules or
/// lines, each with its reason).
enum Keys {
    Only(&'static [&'static str]),
    Any,
}

/// Every table the factory reads.
const TABLES: [(&str, Keys); 16] = [
    ("screenshot", Keys::Only(&["packages"])),
    ("web", Keys::Only(&["package", "product"])),
    ("dev", Keys::Only(&["port", "words", "https"])),
    (
        "serve",
        Keys::Only(&["dir", "https", "path", "check", "check_seconds"]),
    ),
    ("files", Keys::Only(&["max_lines"])),
    ("build", Keys::Only(&["jobs"])),
    ("files.allow", Keys::Any),
    ("files.generated", Keys::Any),
    ("gate", Keys::Only(&["twin"])),
    ("gate.setup", Keys::Any),
    ("gate.actions", Keys::Any),
    ("gate.pinned", Keys::Any),
    ("layers.forbid", Keys::Any),
    ("ux", Keys::Only(&UX_KEYS)),
    ("ux.design", Keys::Any),
    (
        "queue",
        Keys::Only(&["repo", "merge_on", "run", "log", "serve", "targets"]),
    ),
];

/// `[ux]`, read by `ux_config`.
const UX_KEYS: [&str; 16] = [
    "run_dir",
    "checklist",
    "report_dir",
    "model",
    "endpoint",
    "key_env",
    "reasoning",
    "image_detail",
    "price_input",
    "price_cached",
    "price_output",
    "price_source",
    "max_input_tokens",
    "max_output_tokens",
    "max_cost_usd",
    "daily_cost_usd",
];

/// Tables named by a prefix: `[imports.<crate>]`, any key.
const ANY_UNDER: [&str; 1] = ["imports."];

/// Refuse a table or a key the factory does not read.
///
/// # Errors
///
/// The first unknown table or key, at its line.
pub fn check_known(config: &Config) -> Result<(), ConfigError> {
    for (table, line) in config.table_lines() {
        if keys_of(table).is_none() {
            let known: Vec<&str> = TABLES.iter().map(|(name, _)| *name).collect();
            let reason = format!(
                "[{table}] is not a table the factory reads; fix: use one of {}, or [imports.<crate>]",
                known.join(", ")
            );
            return Err(ConfigError { line, reason });
        }
    }
    for placed in config.placed_keys() {
        if let Some(Keys::Only(names)) = keys_of(&placed.table)
            && !names.contains(&placed.key.as_str())
        {
            let reason = format!(
                "{}.{} is not a key the factory reads; fix: use {}",
                placed.table,
                placed.key,
                names.join(", ")
            );
            return Err(ConfigError {
                line: placed.line,
                reason,
            });
        }
    }
    Ok(())
}

fn keys_of(table: &str) -> Option<&'static Keys> {
    if ANY_UNDER.iter().any(|prefix| table.starts_with(prefix)) {
        return Some(&Keys::Any);
    }
    TABLES
        .iter()
        .find(|(name, _)| *name == table)
        .map(|(_, keys)| keys)
}

#[cfg(test)]
#[path = "config_schema_tests.rs"]
mod tests;

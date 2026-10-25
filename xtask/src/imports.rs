//! `cargo xtask check-imports` (the `imports` step): a crate named in
//! `[imports.<crate>]` is named only in the paths listed there, each
//! with its reason. touche keeps `web_sys`, the browser's API, in its
//! web platform code; vapors keeps it in vapors-platform.
//!
//! Declared limit: the check reads source text, so a dependency renamed
//! in Cargo.toml (`dom = { package = "web-sys" }`) is named `dom` and
//! not seen. `[layers.forbid]` guards dependencies themselves.

use std::path::Path;
use std::process::ExitCode;

use crate::config::Config;
use crate::files_walk::all_source_files;
use crate::imports_rules::{Rules, violations};

const FIX: &str = "fix: move the call into an allowed module, or list the path in \
                   .config/factory.toml under [imports.<crate>] with a reason";

/// Check every rule and report.
pub fn run() -> ExitCode {
    match check(Path::new(".")) {
        Ok(said) => {
            println!("{said}");
            ExitCode::SUCCESS
        }
        Err(why) => {
            println!("FAIL imports: {why}");
            ExitCode::FAILURE
        }
    }
}

fn check(root: &Path) -> Result<String, String> {
    let rules = rules(&Config::load().map_err(|e| e.to_string())?)?;
    if rules.is_empty() {
        return Ok("ok   imports  no rules under [imports.<crate>]".to_owned());
    }
    check_paths_exist(root, &rules)?;
    let found = violations(&sources(root)?, &rules);
    if found.is_empty() {
        let names: Vec<&str> = rules.keys().map(String::as_str).collect();
        return Ok(format!(
            "ok   imports  {} named only where allowed",
            names.join(", ")
        ));
    }
    Err(format!("{}\n     {FIX}", found.join("\n     ")))
}

/// `[imports.<crate>]` tables, keyed by crate. A key straight under
/// `[imports]`, a crate name that is not a Rust name, or an empty table
/// is refused.
pub fn rules(config: &Config) -> Result<Rules, String> {
    let mut rules = Rules::new();
    for table in config.tables_under("imports") {
        let name = table_crate(table)?;
        let allow = config.entries_under(&format!("{table}."));
        if allow.is_empty() {
            return Err(format!(
                "[{table}] is empty; fix: list the paths that may name {name}, or remove the table"
            ));
        }
        rules.insert(name.to_owned(), allow);
    }
    Ok(rules)
}

/// The crate a table names: `[imports.<crate>]`, `<crate>` a Rust name.
fn table_crate(table: &str) -> Result<&str, String> {
    let name = table
        .strip_prefix("imports.")
        .ok_or_else(|| format!("[{table}] holds no crate; fix: use [imports.<crate>] tables"))?;
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!(
            "[imports.{name}]: {name} is not a Rust crate name as code names it; fix: write it with underscores, as `use` names it"
        ));
    }
    Ok(name)
}

/// An allowed path that is gone would hide nothing but mislead; refuse it.
pub fn check_paths_exist(root: &Path, rules: &Rules) -> Result<(), String> {
    for (name, allow) in rules {
        if let Some(path) = allow.keys().find(|path| !root.join(path).exists()) {
            return Err(format!(
                "[imports.{name}] lists {path}, which does not exist; fix: update or remove it"
            ));
        }
    }
    Ok(())
}

/// Every `.rs` file in the workspace, with its text.
fn sources(root: &Path) -> Result<Vec<(String, String)>, String> {
    all_source_files(root)?
        .into_iter()
        .filter(|path| path.extension().is_some_and(|e| e == "rs"))
        .map(|path| {
            let text = std::fs::read_to_string(root.join(&path))
                .map_err(|e| format!("{}: {e}", path.display()))?;
            Ok((path.display().to_string(), text))
        })
        .collect()
}

#[cfg(test)]
#[path = "imports_tests.rs"]
mod tests;

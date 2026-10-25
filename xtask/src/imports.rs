//! `cargo xtask check-imports` (the `imports` step): a crate named in
//! `[imports.<crate>]` is named only in the paths listed there, each
//! with its reason. touche keeps `web_sys`, the browser's API, in its
//! web platform code; vapors keeps it in vapors-platform.

use std::path::Path;
use std::process::ExitCode;

use crate::config::Config;
use crate::files_walk::workspace_files;
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
    let rules = rules(&Config::load().map_err(|e| e.to_string())?);
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

/// `[imports.<crate>]` tables, keyed by crate.
pub fn rules(config: &Config) -> Rules {
    let mut rules = Rules::new();
    for (key, reason) in config.entries_under("imports.") {
        if let Some((name, path)) = key.split_once('.') {
            rules
                .entry(name.to_owned())
                .or_default()
                .insert(path.to_owned(), reason);
        }
    }
    rules
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
    workspace_files(root)?
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

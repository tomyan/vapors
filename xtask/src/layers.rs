//! `cargo xtask check-layers` (the `layers` step): no crate depends on a
//! crate its rule in `[layers.forbid]` names, directly or through
//! others. Each rule is `"<crate>" = "<crate>, <crate>, ..."`.

use std::fs::File;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::layers_graph::{Rule, parse, violations};

/// `cargo metadata` took 0.3 to 0.4 s warm on the dev machine
/// (2026-10-25); a cold run may update the registry index first.
const METADATA_DEADLINE: Duration = Duration::from_secs(60);

const FIX: &str = "fix: remove the dependency, or move the code to a crate the rule allows; \
                   reproduce: cargo tree -e normal,build -i <crate> -p <rule's crate>";

/// Check every rule and report.
pub fn run() -> ExitCode {
    match check() {
        Ok(said) => {
            println!("{said}");
            ExitCode::SUCCESS
        }
        Err(why) => {
            println!("FAIL layers: {why}");
            ExitCode::FAILURE
        }
    }
}

fn check() -> Result<String, String> {
    let config = Config::load().map_err(|e| e.to_string())?;
    let rules = rules(&config)?;
    if rules.is_empty() {
        return Ok("ok   layers   no rules in [layers.forbid]".to_owned());
    }
    let graph = parse(&metadata()?)?;
    let found = violations(&graph, &rules)?;
    if found.is_empty() {
        return Ok(format!(
            "ok   layers   {} crates clear of their forbidden crates",
            rules.len()
        ));
    }
    Err(format!("{}\n     {FIX}", found.join("\n     ")))
}

/// The rules in `[layers.forbid]`; an empty crate name is refused.
pub fn rules(config: &Config) -> Result<Vec<Rule>, String> {
    config
        .entries_under("layers.forbid.")
        .into_iter()
        .map(|(crate_name, list)| {
            let forbidden: Vec<String> = list.split(',').map(|n| n.trim().to_owned()).collect();
            if forbidden.iter().any(String::is_empty) {
                return Err(format!(
                    "[layers.forbid] {crate_name} = \"{list}\" has an empty name; fix: list crate names separated by commas"
                ));
            }
            Ok(Rule { crate_name, forbidden })
        })
        .collect()
}

/// `cargo metadata`, through a file: its output is megabytes.
fn metadata() -> Result<serde_json::Value, String> {
    let path = Path::new("target/xtask-metadata.json");
    std::fs::create_dir_all("target").map_err(|e| format!("target: {e}"))?;
    let file = File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut child = Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .stdout(file)
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("could not run cargo metadata: {e}"))?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            if !status.success() {
                return Err(format!(
                    "cargo metadata failed with {status}; reproduce: cargo metadata --format-version 1"
                ));
            }
            break;
        }
        if started.elapsed() >= METADATA_DEADLINE {
            let _ = child.kill();
            return Err(format!(
                "cargo metadata did not finish within {} s",
                METADATA_DEADLINE.as_secs()
            ));
        }
        thread::sleep(Duration::from_millis(50));
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("cargo metadata's output is not JSON: {e}"))
}

#[cfg(test)]
#[path = "layers_tests.rs"]
mod tests;

//! `cargo xtask check-layers` (the `layers` step): no crate depends on a
//! crate its rule in `[layers.forbid]` names, directly or through
//! others. Each rule is `"<crate>" = "<crate>, <crate>, ..."`.

use std::fs::File;
use std::path::PathBuf;
use std::process::{Child, Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::layers_graph::{Rule, parse, violations};

/// `cargo metadata --all-features` took 0.3 to 0.5 s warm on the dev
/// machine and 10.3 to 10.5 s with an empty `CARGO_HOME`, which fetches
/// every manifest (2026-10-25); about 2.5x the cold run.
const METADATA_DEADLINE: Duration = Duration::from_secs(30);

const REPRODUCE: &str = "cargo metadata --format-version 1 --all-features";

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

/// The rules in `[layers.forbid]`. Another table under `layers`, an
/// empty `[layers.forbid]` or an empty crate name is refused.
pub fn rules(config: &Config) -> Result<Vec<Rule>, String> {
    if let Some(table) = config
        .tables_under("layers")
        .into_iter()
        .find(|t| *t != "layers.forbid")
    {
        return Err(format!(
            "[{table}] is not a table check-layers reads; fix: use [layers.forbid]"
        ));
    }
    let entries = config.entries_under("layers.forbid.");
    if entries.is_empty() && !config.tables_under("layers.forbid").is_empty() {
        return Err("[layers.forbid] is empty; fix: add a rule or remove the table".to_owned());
    }
    entries
        .into_iter()
        .map(|(crate_name, list)| rule(crate_name, &list))
        .collect()
}

fn rule(crate_name: String, list: &str) -> Result<Rule, String> {
    let forbidden: Vec<String> = list.split(',').map(|n| n.trim().to_owned()).collect();
    if forbidden.iter().any(String::is_empty) {
        return Err(format!(
            "[layers.forbid] {crate_name} = \"{list}\" has an empty name; fix: list crate names separated by commas"
        ));
    }
    Ok(Rule {
        crate_name,
        forbidden,
    })
}

/// `cargo metadata` with every feature on, so an optional dependency
/// counts, through a file of this process's own: its output is megabytes.
fn metadata() -> Result<serde_json::Value, String> {
    let path = PathBuf::from(format!("target/xtask-metadata-{}.json", std::process::id()));
    std::fs::create_dir_all("target").map_err(|e| format!("target: {e}"))?;
    let file = File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let child = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--all-features"])
        .stdout(file)
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("could not run cargo metadata: {e}"))?;
    let waited = wait(child);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()));
    let _ = std::fs::remove_file(&path);
    waited?;
    serde_json::from_str(&text?).map_err(|e| format!("cargo metadata's output is not JSON: {e}"))
}

/// Wait for `child` by [`METADATA_DEADLINE`]; killed and reaped after it.
fn wait(mut child: Child) -> Result<(), String> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            if status.success() {
                return Ok(());
            }
            return Err(format!(
                "cargo metadata failed with {status}; reproduce: {REPRODUCE}"
            ));
        }
        if started.elapsed() >= METADATA_DEADLINE {
            let _ = child.kill();
            let _ = child.wait();
            let secs = METADATA_DEADLINE.as_secs();
            return Err(format!(
                "cargo metadata did not finish within {secs} s; reproduce: {REPRODUCE}"
            ));
        }
        thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(test)]
#[path = "layers_tests.rs"]
mod tests;

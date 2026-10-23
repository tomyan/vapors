//! The workflow-level parts of the gate's reading of ci.yml: `env:`
//! values, and `defaults.run`, which the gate accepts only as this
//! repository's own checkout path (its root).

use std::collections::{BTreeMap, BTreeSet};

use serde_yaml::Value;

/// A mapping's `env:`, as strings; an expression is refused.
pub fn env_of(value: &Value, label: &str) -> Result<BTreeMap<String, String>, String> {
    let Some(env) = value.get("env").and_then(Value::as_mapping) else {
        return Ok(BTreeMap::new());
    };
    env.iter()
        .map(|(name, value)| {
            let name = name
                .as_str()
                .ok_or_else(|| format!("{label}: an env name that is not a string"))?;
            let text = match value {
                Value::String(text) => text.clone(),
                Value::Number(number) => number.to_string(),
                Value::Bool(flag) => flag.to_string(),
                _ => {
                    return Err(format!(
                        "{label}: env {name} is not a string, number or bool"
                    ));
                }
            };
            if text.contains("${{") {
                return Err(format!(
                    "{label}: env {name} is an expression, which the gate cannot evaluate"
                ));
            }
            Ok((name.to_owned(), text))
        })
        .collect()
}

/// `defaults.run`: only a working directory that is this repository's
/// own checkout path (the gate's root), and no shell.
pub fn check_defaults(doc: &Value) -> Result<(), String> {
    let Some(run) = doc.get("defaults").and_then(|d| d.get("run")) else {
        return Ok(());
    };
    if run.get("shell").is_some() {
        return Err("defaults.run.shell, which the gate does not model".to_owned());
    }
    let Some(directory) = run.get("working-directory").and_then(Value::as_str) else {
        return Ok(());
    };
    if own_checkout_paths(doc).contains(directory) {
        return Ok(());
    }
    Err(format!(
        "defaults.run.working-directory `{directory}` is not this repository's checkout path; the gate runs from the worktree root"
    ))
}

/// Every `path:` of an `actions/checkout` step without `repository:`.
fn own_checkout_paths(doc: &Value) -> BTreeSet<&str> {
    let jobs = doc
        .get("jobs")
        .and_then(Value::as_mapping)
        .into_iter()
        .flat_map(|jobs| jobs.values());
    jobs.flat_map(|job| {
        job.get("steps")
            .and_then(Value::as_sequence)
            .into_iter()
            .flatten()
    })
    .filter(|step| {
        step.get("uses")
            .and_then(Value::as_str)
            .is_some_and(|uses| uses.starts_with("actions/checkout@"))
    })
    .filter(|step| step.get("with").and_then(|w| w.get("repository")).is_none())
    .filter_map(|step| {
        step.get("with")
            .and_then(|w| w.get("path"))
            .and_then(Value::as_str)
    })
    .collect()
}

/// Refuses a `[gate.setup]` line that no run step in `doc` contains: a
/// stale line would skip a step added later with that text.
pub fn check_setup_used(doc: &Value, setup: &BTreeSet<String>) -> Result<(), String> {
    let lines = run_lines(doc);
    match setup.iter().find(|line| !lines.contains(line.as_str())) {
        Some(stale) => Err(format!(
            "[gate.setup] `{stale}` matches no line of a run step in ci.yml; fix: remove it from .config/factory.toml"
        )),
        None => Ok(()),
    }
}

/// Every trimmed line of every `run` in every job.
fn run_lines(doc: &Value) -> BTreeSet<&str> {
    let jobs = doc
        .get("jobs")
        .and_then(Value::as_mapping)
        .into_iter()
        .flatten();
    let steps = jobs
        .filter_map(|(_, job)| job.get("steps").and_then(Value::as_sequence))
        .flatten();
    steps
        .filter_map(|step| step.get("run").and_then(Value::as_str))
        .flat_map(str::lines)
        .map(str::trim)
        .collect()
}

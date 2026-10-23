//! The workflow-level parts of the gate's reading of ci.yml: the keys it
//! models, `env:` values, and where run steps run (`defaults.run`, which
//! the gate accepts only as this repository's own checkout path).

use std::collections::{BTreeMap, BTreeSet};

use serde_yaml::Value;

/// The workflow keys the gate models.
pub const WORKFLOW_KEYS: [&str; 6] = ["name", "on", "permissions", "env", "defaults", "jobs"];

/// Refuses a key of `value` not in `allowed`, naming it and `label`.
pub fn check_keys(value: &Value, allowed: &[&str], label: &str) -> Result<(), String> {
    let map = value
        .as_mapping()
        .ok_or_else(|| format!("{label}: not a mapping"))?;
    for key in map.keys() {
        let key = key
            .as_str()
            .ok_or_else(|| format!("{label}: a key that is not a string"))?;
        if !allowed.contains(&key) {
            return Err(format!(
                "{label}: `{key}:`, which the gate does not model; fix: drop it or teach the gate"
            ));
        }
    }
    Ok(())
}

/// A mapping's `env:`, as strings; an expression or a non-mapping is
/// refused.
pub fn env_of(value: &Value, label: &str) -> Result<BTreeMap<String, String>, String> {
    let Some(env) = value.get("env") else {
        return Ok(BTreeMap::new());
    };
    let env = env
        .as_mapping()
        .ok_or_else(|| format!("{label}: `env:` is not a mapping"))?;
    env.iter()
        .map(|(name, value)| {
            let name = name
                .as_str()
                .ok_or_else(|| format!("{label}: an env name that is not a string"))?;
            Ok((name.to_owned(), env_value(value, name, label)?))
        })
        .collect()
}

fn env_value(value: &Value, name: &str, label: &str) -> Result<String, String> {
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
    Ok(text)
}

/// The workspace root relative to the gate's root (this checkout): `..`
/// once per component of the checkout's `path:`. Every job's run steps
/// must run in the checkout (`defaults.run`, the job's over the
/// workflow's), since the gate runs them from there.
pub fn workspace(doc: &Value, jobs: &serde_yaml::Mapping) -> Result<String, String> {
    let path = own_checkout_path(doc)?;
    let workflow = run_defaults(doc, "the workflow")?;
    for (name, job) in jobs {
        let label = name.as_str().unwrap_or("a job");
        let directory = run_defaults(job, label)?.or(workflow).unwrap_or(".");
        if normal(directory) != path {
            return Err(format!(
                "{label}: run steps run in `{directory}`, not this repository's checkout `{path}`; the gate runs from the checkout"
            ));
        }
    }
    let depth = path.split('/').filter(|part| *part != ".").count();
    Ok(if depth == 0 {
        ".".to_owned()
    } else {
        vec![".."; depth].join("/")
    })
}

/// `defaults.run.working-directory` of `value`; a shell is refused.
fn run_defaults<'a>(value: &'a Value, label: &str) -> Result<Option<&'a str>, String> {
    let Some(defaults) = value.get("defaults") else {
        return Ok(None);
    };
    check_keys(defaults, &["run"], &format!("{label}: defaults"))?;
    let Some(run) = defaults.get("run") else {
        return Ok(None);
    };
    check_keys(
        run,
        &["working-directory"],
        &format!("{label}: defaults.run"),
    )?;
    Ok(run.get("working-directory").and_then(Value::as_str))
}

/// The one `path:` this repository is checked out to (`actions/checkout`
/// without `repository:`), `.` when none is given.
fn own_checkout_path(doc: &Value) -> Result<String, String> {
    let paths: BTreeSet<String> = own_checkouts(doc)
        .map(|step| {
            step.get("with")
                .and_then(|w| w.get("path"))
                .and_then(Value::as_str)
        })
        .map(|path| normal(path.unwrap_or(".")))
        .collect();
    match paths.len() {
        0 => Ok(".".to_owned()),
        1 => Ok(paths.into_iter().next().unwrap_or_default()),
        _ => Err(format!(
            "this repository is checked out to several paths ({paths:?}); the gate models one"
        )),
    }
}

fn own_checkouts(doc: &Value) -> impl Iterator<Item = &Value> {
    let jobs = doc
        .get("jobs")
        .and_then(Value::as_mapping)
        .into_iter()
        .flatten();
    jobs.filter_map(|(_, job)| job.get("steps").and_then(Value::as_sequence))
        .flatten()
        .filter(|step| {
            let uses = step.get("uses").and_then(Value::as_str).unwrap_or_default();
            uses.starts_with("actions/checkout@")
                && step.get("with").and_then(|w| w.get("repository")).is_none()
        })
}

fn normal(path: &str) -> String {
    let trimmed = path.trim_start_matches("./").trim_end_matches('/');
    if trimmed.is_empty() {
        ".".to_owned()
    } else {
        trimmed.to_owned()
    }
}
